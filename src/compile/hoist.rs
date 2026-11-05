//! Loop-invariant code motion: a sum, difference, product or negation inside a loop whose operands the loop
//! never assigns is worked out once, before the loop. A larger program's `tr_add` computed `999 - cl` for every item of
//! its deck, though `cl` was read once before the loop: three instructions an item.
//!
//! Only what can't go wrong when worked out early: `+`, `-`, `*` (wrapping or unchecked: no panic edge) and unary
//! `-`, of constants and whole locals (no projections), into a local assigned nowhere else in the function, none of
//! them with its address taken. Division is left where it is (worked out before a loop that runs no times, a divisor
//! of 0 would hang the division routine). The value goes at the end of the loop's one way in from outside (its
//! preheader); a loop entered from more than one block outside is left alone. The hoisted local's storage markers
//! go (it now lives across the loop). Nested loops: repeated until nothing moves, so an invariant climbs out of each.
use std::collections::{HashMap, HashSet};

use super::blocks::addressed_locals;
use super::ir::{BasicBlock, BinOp, Block, Local, Operand, Rvalue, Statement, UnOp};

thread_local! { static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// Whether hoisting is on: CARDIAC_HOIST=1 (while it's measured), and not turned off for this program.
pub fn enabled() -> bool { !OFF.with(|o| o.get()) && std::env::var_os("CARDIAC_HOIST").is_some_and(|v| v == "1") }

/// Turn it off (or on again) for the programs compiled next on this thread: a program it makes too big for memory is
/// compiled again without it (longer-lived values can cost the cells sharing would have saved).
pub fn set_off(off: bool) { OFF.with(|o| o.set(off)); }

fn operand_local(o: &Operand) -> Result<Option<Local>, ()> {
    match o {
        Operand::Constant(_) => Ok(None),
        Operand::Copy(p) | Operand::Move(p) => p.as_local().map(Some).ok_or(()),
        _ => Err(()),
    }
}

/// The locals a statement reads, if it is one this pass may move: its destination and operands.
fn movable(s: &Statement) -> Option<(Local, Vec<Local>)> {
    let Statement::Assign(dest, rv) = s else { return None };
    let dest = dest.as_local()?;
    let ops: Vec<&Operand> = match rv {
        Rvalue::BinaryOp(op, a, b) if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::AddUnchecked | BinOp::SubUnchecked | BinOp::MulUnchecked) => vec![a, b],
        Rvalue::UnaryOp(UnOp::Neg, a) => vec![a],
        _ => return None,
    };
    let mut reads = vec![];
    for o in ops { if let Some(l) = operand_local(o).ok()? { reads.push(l); } }
    Some((dest, reads))
}

/// Each block's dominators (by name), the entry the first block.
fn dominators(blocks: &[Block], preds: &HashMap<BasicBlock, Vec<BasicBlock>>) -> HashMap<BasicBlock, HashSet<BasicBlock>> {
    let all: HashSet<BasicBlock> = blocks.iter().map(|b| b.name).collect();
    let entry = blocks[0].name;
    let mut dom: HashMap<BasicBlock, HashSet<BasicBlock>> = blocks.iter().map(|b| (b.name, if b.name == entry { [entry].into() } else { all.clone() })).collect();
    loop {
        let mut changed = false;
        for b in blocks.iter().skip(1) {
            let ps = preds.get(&b.name).cloned().unwrap_or_default();
            let mut d: Option<HashSet<BasicBlock>> = None;
            for p in ps { let pd = &dom[&p]; d = Some(match d { None => pd.clone(), Some(x) => x.intersection(pd).copied().collect() }); }
            let mut d = d.unwrap_or_default();
            d.insert(b.name);
            if d != dom[&b.name] { dom.insert(b.name, d); changed = true; }
        }
        if !changed { return dom; }
    }
}

/// Hoist what it can; true if anything moved.
pub fn hoist_invariants(blocks: &mut [Block], fixed: &HashSet<Local>) -> bool {
    let mut any = false;
    while hoist_once(blocks, fixed) { any = true; }
    any
}

fn hoist_once(blocks: &mut [Block], fixed: &HashSet<Local>) -> bool {
    if blocks.is_empty() { return false; }
    let index: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let mut preds: HashMap<BasicBlock, Vec<BasicBlock>> = HashMap::new();
    for b in blocks.iter() { for s in b.terminator.successors() { preds.entry(s).or_default().push(b.name); } }
    let dom = dominators(blocks, &preds);
    let taken = addressed_locals(blocks);
    // how many times each local is assigned, in the whole function
    let mut defs: HashMap<Local, usize> = HashMap::new();
    for b in blocks.iter() { for l in b.each_line() { if let (Some(d), _) = l.def_use() { *defs.entry(d).or_default() += 1; } } }
    for b in blocks.iter() {
        for h in b.terminator.successors() {
            // a back edge b -> h: h dominates b
            if !dom.get(&b.name).is_some_and(|d| d.contains(&h)) { continue; }
            // the loop: h and everything that reaches b without passing h
            let mut body: HashSet<BasicBlock> = [h, b.name].into();
            let mut stack = vec![b.name];
            while let Some(x) = stack.pop() {
                if x == h { continue; }
                for &p in preds.get(&x).map(|v| v.as_slice()).unwrap_or(&[]) { if body.insert(p) { stack.push(p); } }
            }
            let outside: Vec<BasicBlock> = preds.get(&h).cloned().unwrap_or_default().into_iter().filter(|p| !body.contains(p)).collect();
            let [pre] = outside[..] else { continue };
            // what the preheader's terminator defines (a call's result: `x = inp()`) isn't there yet at the end of
            // its statements, where the hoisted value goes
            let late: Option<Local> = blocks[index[&pre]].terminator.def();
            // what the loop assigns
            let mut assigned: HashSet<Local> = HashSet::new();
            for &n in &body { for l in blocks[index[&n]].each_line() { if let (Some(d), _) = l.def_use() { assigned.insert(d); } } }
            // the first movable statement whose operands the loop leaves alone
            let mut found = None;
            'search: for &n in &body {
                for (si, s) in blocks[index[&n]].statements.iter().enumerate() {
                    let Some((dest, reads)) = movable(s) else { continue };
                    if fixed.contains(&dest) || taken.contains(&dest) || defs.get(&dest) != Some(&1) { continue; }
                    if reads.iter().any(|r| assigned.contains(r) || taken.contains(r) || late == Some(*r)) { continue; }
                    found = Some((n, si, dest));
                    break 'search;
                }
            }
            let Some((n, si, dest)) = found else { continue };
            let s = blocks[index[&n]].statements.remove(si);
            blocks[index[&pre]].statements.push(s);
            for blk in blocks.iter_mut() {
                blk.statements.retain(|s| !matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if *l == dest));
            }
            return true;
        }
    }
    false
}
