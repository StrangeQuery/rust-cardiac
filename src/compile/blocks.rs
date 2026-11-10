//! What the passes ask of a function's blocks: what a line defines and reads, what is still to be read after each
//! line, whose address is taken. And two small clean-ups: joining a block to the one it alone jumps to, and
//! dropping statements whose result nobody reads.
use std::collections::{HashMap, HashSet};

use super::ir::{self, predecessors, BasicBlock, Block, Local, Locals, Rvalue, Statement, Terminator};

/// One line of a block.
#[derive(Clone, Copy)]
pub enum Line<'a> {
    Statement(&'a Statement),
    Terminator(&'a Terminator),
}

impl Block {
    /// Its lines, statements then terminator.
    pub fn each_line(&self) -> impl Iterator<Item = Line<'_>> {
        self.statements
            .iter()
            .map(Line::Statement)
            .chain(std::iter::once(Line::Terminator(&self.terminator)))
    }
}

impl Line<'_> {
    pub fn locals(self) -> Vec<Local> {
        match self {
            Line::Statement(s) => s.locals(),
            Line::Terminator(t) => t.locals(),
        }
    }

    /// What it defines (a whole local it assigns, by statement or call) and the locals it reads, in order.
    pub fn def_use(self) -> (Option<Local>, Vec<Local>) {
        let def = match self {
            Line::Statement(s) => s.def(),
            Line::Terminator(t) => t.def(),
        };
        let mut uses = self.locals();
        if def.is_some() {
            uses.remove(0);
        }
        uses.retain(|l| *l != Local::UNIT);
        (def, uses)
    }
}

/// For each block, the set of locals still to be read after each of its lines.
pub fn live_after(blocks: &[Block]) -> Vec<Vec<HashSet<Local>>> {
    live_after_returning(blocks, &HashSet::new())
}

/// The same, with the locals of `results` read by every return: the function's result cells, which its caller reads.
pub fn live_after_returning(blocks: &[Block], results: &HashSet<Local>) -> Vec<Vec<HashSet<Local>>> {
    let mut live_in: HashMap<BasicBlock, HashSet<Local>> = HashMap::new();
    let entering = |b: &Block, live_in: &HashMap<BasicBlock, HashSet<Local>>| -> HashSet<Local> {
        let mut live: HashSet<Local> = b
            .terminator
            .successors()
            .iter()
            .flat_map(|n| live_in.get(n).into_iter().flatten().copied())
            .collect();
        if b.terminator == Terminator::Return {
            live.extend(results);
        }
        live
    };
    let mut changed = true;
    while changed {
        changed = false;
        for b in blocks.iter().rev() {
            let mut live = entering(b, &live_in);
            let lines: Vec<Line> = b.each_line().collect();
            for l in lines.into_iter().rev() {
                let (def, uses) = l.def_use();
                if let Some(d) = def {
                    live.remove(&d);
                }
                live.extend(uses);
            }
            if live_in.get(&b.name) != Some(&live) {
                live_in.insert(b.name, live);
                changed = true;
            }
        }
    }
    blocks
        .iter()
        .map(|b| {
            let mut live = entering(b, &live_in);
            let lines: Vec<Line> = b.each_line().collect();
            let mut sets = vec![HashSet::new(); lines.len()];
            for i in (0..lines.len()).rev() {
                sets[i] = live.clone();
                let (def, uses) = lines[i].def_use();
                if let Some(d) = def {
                    live.remove(&d);
                }
                live.extend(uses);
            }
            sets
        })
        .collect()
}

/// `move x` says x is not read again (its last read): the lowering trusts it, and a compare whose bool is moved
/// into a switch is one branch that never stores the bool. A pass that renames a local can break that: `_5 = copy
/// _6; switchInt(move _5)` forwarded is `switchInt(move _6)`, with _6 still tested later. So a move of
/// a whole local still to be read after its line (and not set by that line) becomes a copy. (A move of one field,
/// `move (_5.0)` with `_5.1` read later, is a field's last read, as rustc writes it.)
pub fn copy_unless_last(blocks: &mut [Block]) {
    let live = live_after(blocks);
    for (b, live) in blocks.iter_mut().zip(live) {
        let n = b.statements.len();
        for (i, after) in live.iter().enumerate() {
            let def = if i < n {
                b.statements[i].def()
            } else {
                b.terminator.def()
            };
            let operands = if i < n {
                b.statements[i].operands_mut()
            } else {
                b.terminator.operands_mut()
            };
            for o in operands {
                if let ir::Operand::Move(p) = o {
                    if p.as_local().is_some_and(|l| after.contains(&l) && def != Some(l)) {
                        *o = ir::Operand::Copy(p.clone());
                    }
                }
            }
        }
    }
}

/// The locals whose address is taken anywhere: `&x`, `&mut x[i]`, `&(x.0: T)`, `&(*x)`, with at most one
/// projection wrapped around the local.
pub fn addressed_locals(blocks: &[Block]) -> HashSet<Local> {
    let mut taken = HashSet::new();
    for s in blocks.iter().flat_map(|b| &b.statements) {
        if let Statement::Assign(_, Rvalue::Ref(_, p)) = s {
            if let (Some(l), true) = (p.base_local(), p.wrapped() <= 1) {
                taken.insert(l);
            }
        }
    }
    taken
}

/// Block merging: a block that ends `goto -> S`, where nothing else leads to S, takes S's statements and
/// terminator in place of its goto.
pub fn merge_blocks(blocks: &mut Vec<ir::Block>) {
    let entry = ir::BasicBlock(0);
    loop {
        let found = (0..blocks.len()).find_map(|bi| {
            let ir::Terminator::Goto(s) = blocks[bi].terminator else {
                return None;
            };
            if s == blocks[bi].name || s == entry || blocks.iter().filter(|b| b.terminator.names(s)).count() != 1 {
                return None;
            }
            let si = blocks.iter().position(|b| b.name == s)?;
            (si != bi).then_some((bi, si))
        });
        let Some((bi, si)) = found else { break };
        let taken = blocks.remove(si);
        let bi = if si < bi { bi - 1 } else { bi };
        blocks[bi].statements.extend(taken.statements);
        blocks[bi].terminator = taken.terminator;
    }
}

/// A comparison turned over, `b = Eq(x, y); c = Not(move b)` (a derived `!=`, or `!(x < y)`), is the opposite
/// comparison, `c = Ne(x, y)`, when nothing else reads b: then a branch on c can test x and y itself, where b would
/// be stored as 0 or 1, turned over and tested again.
pub fn not_of_comparisons(blocks: &mut [ir::Block]) {
    let opposite = |op: ir::BinOp| {
        use ir::BinOp::*;
        Some(match op {
            Eq => Ne,
            Ne => Eq,
            Lt => Ge,
            Ge => Lt,
            Le => Gt,
            Gt => Le,
            _ => return None,
        })
    };
    for bi in 0..blocks.len() {
        let mut i = 1;
        while i < blocks[bi].statements.len() {
            let ss = &blocks[bi].statements;
            let folded = match (&ss[i - 1], &ss[i]) {
                (
                    Statement::Assign(b, Rvalue::BinaryOp(op, x, y)),
                    Statement::Assign(c, Rvalue::UnaryOp(ir::UnOp::Not, ir::Operand::Copy(n) | ir::Operand::Move(n))),
                ) if b.as_local().is_some() && n == b => {
                    opposite(*op).map(|op| (b.clone(), c.clone(), op, x.clone(), y.clone()))
                }
                _ => None,
            };
            if let Some((b, c, op, x, y)) = folded {
                let b = b.as_local().unwrap();
                if ir::occurrences(blocks, b) == 2 {
                    blocks[bi].statements[i] = Statement::Assign(c, Rvalue::BinaryOp(op, x, y));
                    blocks[bi].statements.remove(i - 1);
                    continue;
                }
            }
            i += 1;
        }
    }
}

/// A comparison tested at once and read again later: `b = Gt(x, y); switchInt(copy b) -> [0: no, otherwise: yes]`,
/// with b still to be read on a way on (`let e = x > y; if !e { ..e = ..; } if e { .. }`). Each way the
/// branch goes knows b, so a way where it is still read sets it there, as the constant it is (a block of its own
/// on the edge), and the branch reads b for the last time: it is one branch that never stores b, and
/// `thread_constants` takes a constant tested again straight to the arm it picks. With `threaded_only` (before
/// `thread_constants`), only where every way that reads b goes straight to a test of b, which threading then takes
/// past: elsewhere b may yet be worked out again where it is read (`arith::rematerialize`), which costs no cell.
pub fn bools_on_edges(blocks: &mut Vec<ir::Block>, threaded_only: bool) {
    use ir::{BinOp, Const, Operand, Place};
    let live = live_after(blocks);
    // what is still to be read on entering each block
    let live_in: HashMap<BasicBlock, HashSet<Local>> = blocks
        .iter()
        .zip(&live)
        .map(|(b, after)| {
            let mut l = after[0].clone();
            let (def, uses) = b.each_line().next().unwrap().def_use();
            if let Some(d) = def {
                l.remove(&d);
            }
            l.extend(uses);
            (b.name, l)
        })
        .collect();
    let taken = addressed_locals(blocks);
    let mut fresh = 1 + blocks.iter().map(|b| b.name.0).max().unwrap_or(0);
    // the blocks that set a bool and go on, `x = const v; goto -> t`: an edge that needs one takes it
    let mut setting: HashMap<(Local, bool, BasicBlock), BasicBlock> = HashMap::new();
    for b in blocks.iter() {
        if let ([Statement::Assign(p, Rvalue::Use(Operand::Constant(Const::Bool(v))))], Terminator::Goto(t)) =
            (b.statements.as_slice(), &b.terminator)
        {
            if let Some(x) = p.as_local() {
                setting.insert((x, *v, *t), b.name);
            }
        }
    }
    // the blocks that do nothing but test a local
    let tests: HashMap<BasicBlock, Local> = blocks
        .iter()
        .filter_map(|b| match (&b.terminator, b.statements.is_empty()) {
            (
                Terminator::SwitchInt {
                    discr: Operand::Copy(p) | Operand::Move(p),
                    ..
                },
                true,
            ) => p.as_local().map(|x| (b.name, x)),
            _ => None,
        })
        .collect();
    let mut edges = vec![];
    for b in blocks.iter_mut() {
        let Some(Statement::Assign(d, Rvalue::BinaryOp(op, ..))) = b.statements.last() else {
            continue;
        };
        if !matches!(
            op,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        ) {
            continue;
        }
        let Some(x) = d.as_local().filter(|x| *x != Local::UNIT && !taken.contains(x)) else {
            continue;
        };
        let Terminator::SwitchInt {
            discr: discr @ (Operand::Copy(_) | Operand::Move(_)),
            arms,
            otherwise: Some(yes),
        } = &mut b.terminator
        else {
            continue;
        };
        let ([(0, no)], true) = (arms.as_mut_slice(), discr.place().and_then(Place::as_local) == Some(x)) else {
            continue;
        };
        let read_on = |t: &BasicBlock| live_in.get(t).is_some_and(|l| l.contains(&x));
        if *yes == *no || !(read_on(yes) || read_on(no)) {
            continue;
        }
        let tests_x = |t: &BasicBlock| tests.get(t) == Some(&x);
        if threaded_only && [&*yes, &*no].iter().any(|t| read_on(t) && !tests_x(t)) {
            continue;
        }
        for (to, v) in [(yes, true), (no, false)] {
            if let (true, Some(&edge)) = (read_on(to), setting.get(&(x, v, *to))) {
                *to = edge;
            } else if read_on(to) {
                let edge = BasicBlock(fresh);
                setting.insert((x, v, *to), edge);
                fresh += 1;
                edges.push((
                    *to,
                    Block {
                        name: edge,
                        statements: vec![Statement::Assign(
                            Place::local(x),
                            Rvalue::Use(Operand::Constant(Const::Bool(v))),
                        )],
                        terminator: Terminator::Goto(*to),
                    },
                ));
                *to = edge;
            }
        }
        *discr = Operand::Move(Place::local(x));
    }
    // each just before the block it goes to, which it falls into (never before the entry)
    for (to, edge) in edges {
        let at = blocks
            .iter()
            .position(|b| b.name == to)
            .filter(|at| *at > 0)
            .unwrap_or(blocks.len());
        blocks.insert(at, edge);
    }
}

/// Dead statements: a local assigned by statements and read by nothing but its own assignments is not worked out
/// at all. Runs to a fixed point (removing one can leave another unread); each round counts the uses as they were
/// when it began. The result, and a struct result's fields, are read by the caller.
/// The local a place is one field of, `(_26.0: usize)`.
fn field_of(p: &ir::Place) -> Option<ir::Local> {
    match p.projection.as_slice() {
        [ir::Projection::Field(..)] => p.base_local(),
        _ => None,
    }
}

pub fn dead_code(blocks: &mut [ir::Block], results: &HashSet<ir::Local>) {
    loop {
        // the lines that name each local, other than a statement assigning the whole of it
        let mut reading: HashMap<ir::Local, usize> = HashMap::new();
        for b in blocks.iter() {
            for line in b.each_line() {
                // (a field of one too, `(_26.0: usize) = ...`, as the range pass leaves a checked op it proved)
                let assigned = match line {
                    Line::Statement(ir::Statement::Assign(p, _)) => p.as_local().or_else(|| field_of(p)),
                    _ => None,
                };
                for l in line.locals().into_iter().collect::<HashSet<_>>() {
                    if Some(l) != assigned {
                        *reading.entry(l).or_default() += 1;
                    }
                }
            }
        }
        let mut changed = false;
        for b in blocks.iter_mut() {
            b.statements.retain(|s| {
                let ir::Statement::Assign(p, _) = s else { return true };
                let Some(l) = p.as_local().or_else(|| field_of(p)) else {
                    return true;
                };
                let dead =
                    l != ir::Local(0) && l != ir::Local::UNIT && !results.contains(&l) && !reading.contains_key(&l);
                changed |= dead;
                !dead
            });
        }
        if !changed {
            break;
        }
    }
}

/// Bounds checks of one index that come one after another: `assert(r < 6)`, then `r + 1 < 6` and `r + 2 < 6` on
/// the way on, as `b[r][c] + b[r + 1][c] + b[r + 2][c]` makes them. Once `r < 6` holds, r is on a card and `r + k < N`
/// holds exactly when `r < N - k`, so the first check can be the strongest, `r < 4`, and the rest are gotos. The
/// program panics on the same inputs, with the same message: the way from the first check to each later one is
/// blocks with one way in, each going on from an assert, with no call (no output) and no other check that could
/// fail first with another message. A bounds check of another index may come between (it fails with the same
/// message), and so may the overflow check of `r + k` itself, which can't fail once r is on a card. The first
/// check's bool must be read by its assert only, and r set nowhere on the way.
pub fn merged_checks(blocks: &mut [ir::Block]) {
    let preds = predecessors(blocks);
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let usize_const = |o: &ir::Operand| match o {
        ir::Operand::Constant(ir::Const::Int {
            value,
            ty: ir::IntTy::Usize,
        }) => Some(*value),
        _ => None,
    };
    let local = |o: &ir::Operand| match o {
        ir::Operand::Copy(p) | ir::Operand::Move(p) => p.as_local(),
        _ => None,
    };
    let reads = |l: ir::Local| -> usize {
        blocks
            .iter()
            .map(|b| b.statements.iter().map(|s| s.count(l)).sum::<usize>() + b.terminator.count(l))
            .sum()
    };
    // a block's bounds check: the index local it compares and the length, where its bool is `v < N` set in the block
    let check = |b: &ir::Block| -> Option<(ir::Local, ir::Local, i128)> {
        let Terminator::Assert {
            cond, expected: true, ..
        } = &b.terminator
        else {
            return None;
        };
        let bc = local(cond)?;
        let k = b
            .statements
            .iter()
            .rposition(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(bc)))?;
        let Statement::Assign(d, Rvalue::BinaryOp(ir::BinOp::Lt, v, n)) = &b.statements[k] else {
            return None;
        };
        let v = local(v)?;
        let unset_after = !b.statements[k + 1..]
            .iter()
            .any(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(v)));
        (d.as_local() == Some(bc) && unset_after).then_some((bc, v, usize_const(n)?))
    };
    let mut edits: Vec<(usize, i128, Vec<usize>)> = vec![];
    for (i0, b0) in blocks.iter().enumerate() {
        let Some((bool0, x, n0)) = check(b0) else { continue };
        let Terminator::Assert { target, .. } = &b0.terminator else {
            continue;
        };
        // the first check's bool read by its assert alone (and set once)
        if reads(bool0) != 2 {
            continue;
        }
        // each local set on the way, as it is set
        let mut defs: HashMap<ir::Local, &Rvalue> = HashMap::new();
        let mut bound = n0;
        let mut members = vec![];
        let (mut prev, mut next) = (b0.name, *target);
        for _ in 0..64 {
            let Some(&j) = at.get(&next) else { break };
            let ins = preds.get(&next).map(Vec::as_slice).unwrap_or_default();
            if ins != [prev] || j == i0 {
                break;
            }
            let b = &blocks[j];
            let mut sets_x = false;
            for s in &b.statements {
                if let Statement::Assign(d, rv) = s {
                    match d.as_local() {
                        Some(l) => {
                            defs.insert(l, rv);
                        }
                        None => {
                            defs.extend(d.base_local().map(|l| (l, rv)));
                        }
                    }
                    sets_x |= d.base_local() == Some(x);
                }
            }
            if sets_x {
                break;
            }
            // what x + k the index v is, k a constant
            let plus = |v: ir::Local| -> Option<i128> {
                if v == x {
                    return Some(0);
                }
                match defs.get(&v)? {
                    Rvalue::Use(o) if local(o) == Some(x) => Some(0),
                    Rvalue::BinaryOp(ir::BinOp::Add, a, k) if local(a) == Some(x) => usize_const(k),
                    Rvalue::BinaryOp(ir::BinOp::Add, k, a) if local(a) == Some(x) => usize_const(k),
                    Rvalue::Use(ir::Operand::Copy(t) | ir::Operand::Move(t))
                        if matches!(t.projection.as_slice(), [ir::Projection::Field(0, _)]) =>
                    {
                        match defs.get(&t.base_local()?)? {
                            Rvalue::BinaryOp(ir::BinOp::AddWithOverflow, a, k) if local(a) == Some(x) => usize_const(k),
                            Rvalue::BinaryOp(ir::BinOp::AddWithOverflow, k, a) if local(a) == Some(x) => usize_const(k),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            };
            let Terminator::Assert { target, msg, .. } = &b.terminator else {
                break;
            };
            match check(b) {
                Some((_, v, n)) => {
                    if let Some(k) = plus(v) {
                        bound = bound.min(n - k);
                        members.push(j);
                    } else if !msg.starts_with("index out of bounds") {
                        break;
                    }
                }
                None => {
                    // the overflow check of x + k, which can't fail once x is on a card
                    let Terminator::Assert {
                        cond: ir::Operand::Copy(t) | ir::Operand::Move(t),
                        expected: false,
                        ..
                    } = &b.terminator
                    else {
                        break;
                    };
                    let pair = match (t.base_local(), t.projection.as_slice()) {
                        (Some(p), [ir::Projection::Field(1, _)]) => p,
                        _ => break,
                    };
                    match defs.get(&pair) {
                        Some(
                            Rvalue::BinaryOp(ir::BinOp::AddWithOverflow, a, k)
                            | Rvalue::BinaryOp(ir::BinOp::AddWithOverflow, k, a),
                        ) if local(a) == Some(x) && usize_const(k).is_some_and(|k| (0..=99).contains(&k)) => {}
                        _ => break,
                    }
                }
            }
            prev = next;
            next = *target;
        }
        if !members.is_empty() {
            edits.push((i0, bound.max(0), members));
        }
    }
    // (a check another run takes is no first check of a run of its own)
    let taken: HashSet<usize> = edits.iter().flat_map(|(_, _, m)| m.iter().copied()).collect();
    for (i0, bound, members) in edits {
        if taken.contains(&i0) {
            continue;
        }
        let Some((bool0, _, _)) = check(&blocks[i0]) else {
            continue;
        };
        for s in blocks[i0].statements.iter_mut().rev() {
            if let Statement::Assign(d, Rvalue::BinaryOp(ir::BinOp::Lt, _, n)) = s {
                if d.as_local() == Some(bool0) {
                    *n = ir::Operand::Constant(ir::Const::Int {
                        value: bound,
                        ty: ir::IntTy::Usize,
                    });
                    break;
                }
            }
        }
        for j in members {
            if let Terminator::Assert { target, .. } = &blocks[j].terminator {
                blocks[j].terminator = Terminator::Goto(*target);
            }
        }
    }
}

/// The usizes a promise keeps on a card: x set once, `assume(x < k)` (k on a card) made on every way from there to
/// any use of x that could be seen. With a promise of x's one value, x is never a usize past a card where the program
/// means anything (where it isn't, the program is undefined), so lowering needs no test of its sign. Where x is read
/// before the promise, only by statements (which compute, and show nothing), the promise covers those too, since x
/// never changes; a branch, check or call on x, or on what is worked out from it, before the promise is not
/// covered, as the program could stop there without making it.
pub fn promised_cards(blocks: &[ir::Block]) -> HashSet<ir::Local> {
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let local = |o: &ir::Operand| match o {
        ir::Operand::Copy(p) | ir::Operand::Move(p) => p.as_local(),
        _ => None,
    };
    let card =
        |o: &ir::Operand| matches!(o, ir::Operand::Constant(ir::Const::Int { value, .. }) if (0..=999).contains(value));
    let mut defs: HashMap<ir::Local, usize> = HashMap::new();
    for b in blocks {
        for s in &b.statements {
            if let Statement::Assign(d, _) = s {
                *defs.entry(d.base_local().unwrap()).or_default() += 1;
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            *defs.entry(dest.base_local().unwrap()).or_default() += 1;
        }
    }
    // each promise: its block, the bool it promises, and how many of the block's lines come before it (an assert
    // that the promise has become, after them all; or still the `assume` statement rustc wrote, as `nested` sees it)
    let mut sites: Vec<(usize, &ir::Operand, usize)> = vec![];
    for (pi, b) in blocks.iter().enumerate() {
        if let Terminator::Assert {
            cond,
            expected: true,
            msg,
            ..
        } = &b.terminator
        {
            if msg == "assumed" {
                sites.push((pi, cond, b.statements.len()));
            }
        }
        for (k, st) in b.statements.iter().enumerate() {
            if let Statement::Assume(cond) = st {
                sites.push((pi, cond, k));
            }
        }
    }
    let mut out = HashSet::new();
    for (pi, cond, end) in sites {
        let b = &blocks[pi];
        let Some(v) = local(cond) else { continue };
        // the comparison promised: `y < k`, `y <= k`, `k > y` or `k >= y`, y a copy of x or x itself
        let def_of = |l: ir::Local| {
            b.statements[..end].iter().rev().find_map(|s| match s {
                Statement::Assign(d, rv) if d.as_local() == Some(l) => Some(rv),
                _ => None,
            })
        };
        let bound = |k: &ir::Operand| match k {
            ir::Operand::Constant(ir::Const::Int { value, .. }) if (0..=999).contains(value) => Some(*value),
            _ => None,
        };
        let (y, k) = match def_of(v) {
            Some(Rvalue::BinaryOp(ir::BinOp::Lt | ir::BinOp::Le, a, k)) if card(k) => (local(a), bound(k)),
            Some(Rvalue::BinaryOp(ir::BinOp::Gt | ir::BinOp::Ge, k, a)) if card(k) => (local(a), bound(k)),
            _ => (None, None),
        };
        let (Some(y), Some(k)) = (y, k) else { continue };
        // y a copy of x, or x itself, or x / d (`n / 10 < 3`: n is below 30, so on a card), where the promised
        // quotient bound times d stays on a card
        let x = match def_of(y) {
            Some(Rvalue::Use(o)) => local(o).unwrap_or(y),
            Some(Rvalue::BinaryOp(ir::BinOp::Div, a, d)) => match (local(a), bound(d)) {
                (Some(a), Some(d)) if d >= 1 && (k + 1) * d <= 1000 => a,
                _ => continue,
            },
            _ => y,
        };
        if defs.get(&x) != Some(&1) {
            continue;
        }
        // the blocks reached from the entry without passing the promise's block, and what is worked out from x
        // there: none of it may be read by a terminator
        let mut seen: HashSet<usize> = HashSet::new();
        let mut todo = vec![0usize];
        while let Some(i) = todo.pop() {
            if i == pi || !seen.insert(i) {
                continue;
            }
            todo.extend(
                blocks[i]
                    .terminator
                    .successors()
                    .iter()
                    .filter_map(|n| at.get(n).copied()),
            );
        }
        let mut tainted: HashSet<ir::Local> = HashSet::from([x]);
        loop {
            let before = tainted.len();
            for &i in &seen {
                for s in &blocks[i].statements {
                    if let Statement::Assign(d, _) = s {
                        if tainted.iter().any(|t| s.count(*t) > 0) {
                            tainted.extend(d.base_local());
                        }
                    }
                }
            }
            if tainted.len() == before {
                break;
            }
        }
        // (another promise reads it as it likes: it is no code, and shows nothing)
        let shows = |t: &Terminator| !matches!(t, Terminator::Assert { msg, .. } if msg == "assumed");
        if !seen
            .iter()
            .any(|&i| shows(&blocks[i].terminator) && tainted.iter().any(|t| blocks[i].terminator.count(*t) > 0))
        {
            out.insert(x);
        }
    }
    out
}

thread_local! {
    /// Whether `reroll_reads` is on: only in a compile that didn't fit without it (see `compile_mir`), since the
    /// loop it makes is smaller but slower.
    pub static REROLL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A sum of reads of one array a constant apart, `t1 = copy a[x]; t2 = t1 + a[x + 1]; ...; t8 = t7 + a[x + 22]`, as
/// the eight neighbours of a square make it: a loop over a table of the offsets, as a program for CARDIAC written by
/// hand walks one. Only six reads or more (fewer cost less written out), of an array that only ever holds constants
/// (`ranges::constant_tables`) whose sum can't pass a card, so each partial sum is the same either way; each sum but
/// the last read once, by the next. The offsets' table is a constant array, so its cells are its initial values.
pub fn reroll_reads(blocks: &mut Vec<ir::Block>, locals: &mut crate::compile::util::Ordered<ir::Local, ir::Ty>) {
    use ir::{ArrayLen, BinOp, Const, IntTy, Local, Operand, Place, PlaceBase, Projection, Ty};
    if !REROLL.with(|r| r.get()) {
        return;
    }
    let tables = super::ranges::constant_tables(blocks, &super::ranges::taken_locals(blocks));
    let mut next_local = locals.keys().map(|l| l.0).max().unwrap_or(0) + 1;
    let mut next_block = blocks.iter().map(|b| b.name.0).max().unwrap_or(0).max(850_000) + 1;
    let mut fresh = |locals: &mut crate::compile::util::Ordered<Local, Ty>, ty: Ty| {
        let l = Local(next_local);
        next_local += 1;
        locals.set(l, ty);
        l
    };
    let reads = |blocks: &[ir::Block], l: Local| -> usize {
        blocks
            .iter()
            .map(|b| b.statements.iter().map(|s| s.count(l)).sum::<usize>() + b.terminator.count(l))
            .sum()
    };
    // an element read `copy a[x]` or `copy a[x+k]`: the array, x and k
    let element = |o: &Operand| -> Option<(Local, Local, u32)> {
        let (Operand::Copy(p) | Operand::Move(p)) = o else {
            return None;
        };
        match (p.base_local(), p.projection.as_slice()) {
            (Some(a), [Projection::Index(x)]) => Some((a, *x, 0)),
            (Some(a), [Projection::IndexPlus(x, k)]) => Some((a, *x, *k)),
            _ => None,
        }
    };
    let mut bi = 0;
    while bi < blocks.len() {
        let mut found: Option<(usize, usize, Local, Local, Vec<u32>, Local)> = None;
        // the sum as one linear combination, `Lin(+1*copy a[x], +1*copy _24, ...)`, each term a read or a local set
        // from one (read nowhere else) earlier in the block: those lines go, and the sum is the line it is on
        for li in 0..blocks[bi].statements.len() {
            let Statement::Assign(d, Rvalue::Lin(terms)) = &blocks[bi].statements[li] else {
                continue;
            };
            let Some(last) = d.as_local() else { continue };
            let mut gone: Vec<usize> = vec![];
            let mut offsets = vec![];
            let mut array: Option<(Local, Local)> = None;
            let mut fine = terms.len() >= 6;
            for (c, o) in terms {
                let read = element(o).map(|r| (r, None)).or_else(|| {
                    let u = o.local()?;
                    let at = blocks[bi].statements[..li]
                        .iter()
                        .rposition(|s| matches!(s, Statement::Assign(d, _) if d.as_local() == Some(u)))?;
                    let Statement::Assign(_, Rvalue::Use(e)) = &blocks[bi].statements[at] else {
                        return None;
                    };
                    (reads(blocks, u) == 2).then_some(())?;
                    element(e).map(|r| (r, Some(at)))
                });
                let Some(((a, x, k), at)) = read.filter(|_| *c == 1) else {
                    fine = false;
                    break;
                };
                if array.is_some_and(|ax| ax != (a, x)) || x == last {
                    fine = false;
                    break;
                }
                array = Some((a, x));
                offsets.push(k);
                gone.extend(at);
            }
            let Some((a, x)) = array.filter(|_| fine) else { continue };
            // (x unset between the reads that go and the sum)
            let first = gone.iter().copied().min().unwrap_or(li);
            let unset = !blocks[bi].statements[first..li]
                .iter()
                .any(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(x)));
            let n = offsets.len() as i64;
            let Some(r) = tables.get(&a) else { continue };
            if unset
                && r[0].abs().max(r[1].abs()).saturating_mul(n) <= 999
                && matches!(locals.get(&last), Some(Ty::Int(IntTy::I16)))
            {
                gone.sort_unstable();
                for at in gone.iter().rev() {
                    blocks[bi].statements.remove(*at);
                }
                let li = li - gone.len();
                found = Some((li, li + 1, a, x, offsets, last));
                break;
            }
        }
        'search: for s0 in 0..blocks[bi].statements.len() {
            if found.is_some() {
                break;
            }
            let Statement::Assign(d0, Rvalue::Use(o0)) = &blocks[bi].statements[s0] else {
                continue;
            };
            let (Some(t0), Some((a, x, k0))) = (d0.as_local(), element(o0)) else {
                continue;
            };
            let Some(r) = tables.get(&a) else { continue };
            let mut offsets = vec![k0];
            let mut last = t0;
            let mut end = s0 + 1;
            while let Some(Statement::Assign(d, Rvalue::BinaryOp(BinOp::Add, p, q))) = blocks[bi].statements.get(end) {
                let (Some(t), Some((a2, x2, k))) = (d.as_local(), element(q)) else {
                    break;
                };
                if p.local() != Some(last) || a2 != a || x2 != x || t == x || reads(blocks, last) != 2 {
                    break;
                }
                offsets.push(k);
                last = t;
                end += 1;
            }
            let n = offsets.len() as i64;
            let fits = r[0].abs().max(r[1].abs()).saturating_mul(n) <= 999;
            if n >= 6 && fits && matches!(locals.get(&last), Some(Ty::Int(IntTy::I16))) {
                found = Some((s0, end, a, x, offsets, last));
                break 'search;
            }
        }
        let Some((s0, end, a, x, offsets, last)) = found else {
            bi += 1;
            continue;
        };
        let n = offsets.len();
        let usize_ty = Ty::Int(IntTy::Usize);
        let table = fresh(locals, Ty::Array(Box::new(usize_ty.clone()), ArrayLen::Known(n as u64)));
        let (acc, q) = (fresh(locals, Ty::Int(IntTy::I16)), fresh(locals, usize_ty.clone()));
        let (w, idx, e) = (
            fresh(locals, usize_ty.clone()),
            fresh(locals, usize_ty.clone()),
            fresh(locals, Ty::Int(IntTy::I16)),
        );
        let more = fresh(locals, Ty::Bool);
        let (body, after) = (ir::BasicBlock(next_block), ir::BasicBlock(next_block + 1));
        next_block += 2;
        let k = |v: i128, ty: IntTy| Operand::Constant(Const::Int { value: v, ty });
        let copy = |l: Local| Operand::Copy(Place::local(l));
        let at = |arr: Local, i: Local| Place {
            base: PlaceBase::Local(arr),
            projection: vec![Projection::Index(i)],
        };
        let b = &mut blocks[bi];
        let rest: Vec<Statement> = b.statements.split_off(end);
        b.statements.truncate(s0);
        b.statements.push(Statement::Assign(
            Place::local(table),
            Rvalue::Array(offsets.iter().map(|o| k(*o as i128, IntTy::Usize)).collect()),
        ));
        b.statements
            .push(Statement::Assign(Place::local(acc), Rvalue::Use(k(0, IntTy::I16))));
        b.statements
            .push(Statement::Assign(Place::local(q), Rvalue::Use(k(0, IntTy::Usize))));
        let terminator = std::mem::replace(&mut b.terminator, Terminator::Goto(body));
        let looped = ir::Block {
            name: body,
            statements: vec![
                Statement::Assign(Place::local(w), Rvalue::Use(Operand::Copy(at(table, q)))),
                Statement::Assign(Place::local(idx), Rvalue::BinaryOp(BinOp::Add, copy(x), copy(w))),
                Statement::Assign(Place::local(e), Rvalue::Use(Operand::Copy(at(a, idx)))),
                Statement::Assign(Place::local(acc), Rvalue::BinaryOp(BinOp::Add, copy(acc), copy(e))),
                Statement::Assign(
                    Place::local(q),
                    Rvalue::BinaryOp(BinOp::Add, copy(q), k(1, IntTy::Usize)),
                ),
                Statement::Assign(
                    Place::local(more),
                    Rvalue::BinaryOp(BinOp::Lt, copy(q), k(n as i128, IntTy::Usize)),
                ),
            ],
            terminator: Terminator::SwitchInt {
                discr: Operand::Move(Place::local(more)),
                arms: vec![(0, after)],
                otherwise: Some(body),
            },
        };
        let mut tail = vec![Statement::Assign(Place::local(last), Rvalue::Use(copy(acc)))];
        tail.extend(rest);
        let done = ir::Block {
            name: after,
            statements: tail,
            terminator,
        };
        blocks.insert(bi + 1, looped);
        blocks.insert(bi + 2, done);
        bi += 3;
    }
}

/// A walk over a small window of a table, `for i in r..r + 3 { for j in c..c + 3 { n += b[i][j] } }` (or one
/// loop), as one loop over a table of the offsets, row after row: `n += b[base + T[q]]` for q in 0..9, base `10 * r +
/// c`, T `[0, 1, 2, 10, 11, 12, 20, 21, 22]`. Each loop runs from a usize a promise keeps on a card (so `r + 3`
/// doesn't wrap: it runs 3 times), and its body, once the range pass has proved its checks, is the index worked
/// out from the loops' variables and what doesn't change, and the one sum: the same reads, in the same order, into
/// the same local, so the sum is the same at every step. Only where `REROLL` is on, as for `reroll_reads`.
pub fn window_sums(
    blocks: &mut Vec<ir::Block>,
    locals: &mut crate::compile::util::Ordered<ir::Local, ir::Ty>,
    promised: &HashSet<ir::Local>,
) {
    if !REROLL.with(|r| r.get()) {
        return;
    }
    while window_sum(blocks, locals, promised) {}
}

/// A loop `for v in a..a + K`, at its header: `cmp = Lt(v, end)`, then on to the body or out.
struct Counted {
    header: usize,
    body: usize,
    exit: BasicBlock,
    v: ir::Local,
    end: ir::Local,
    cmp: ir::Local,
}

/// An affine value: a constant, and each local's coefficient.
type Affine = (i128, std::collections::BTreeMap<ir::Local, i128>);

fn window_sum(
    blocks: &mut Vec<ir::Block>,
    locals: &mut crate::compile::util::Ordered<ir::Local, ir::Ty>,
    promised: &HashSet<ir::Local>,
) -> bool {
    use ir::{ArrayLen, BinOp, Const, IntTy, Local, Operand, Place, PlaceBase, Projection, Ty};
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let preds = predecessors(blocks);
    let local = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    let int = |o: &Operand| match o {
        Operand::Constant(Const::Int { value, .. }) => Some(*value),
        _ => None,
    };
    let counted = |h: usize| -> Option<Counted> {
        let b = &blocks[h];
        let [Statement::Assign(d, Rvalue::BinaryOp(BinOp::Lt, v, end))] = b.statements.as_slice() else {
            return None;
        };
        let Terminator::SwitchInt {
            discr,
            arms,
            otherwise: Some(body),
        } = &b.terminator
        else {
            return None;
        };
        let [(0, exit)] = arms.as_slice() else { return None };
        let cmp = d.as_local()?;
        (local(discr) == Some(cmp)).then_some(())?;
        Some(Counted {
            header: h,
            body: *at.get(body)?,
            exit: *exit,
            v: local(v)?,
            end: local(end)?,
            cmp,
        })
    };
    // a usize that can't be past a card: promised, or a promised one's quotient or remainder by a constant (`s / 10`)
    let single_defs: HashMap<Local, &Rvalue> = {
        let mut all: HashMap<Local, Vec<&Rvalue>> = HashMap::new();
        for b in blocks.iter() {
            for st in &b.statements {
                if let Statement::Assign(d, rv) = st {
                    all.entry(d.base_local().unwrap()).or_default().push(rv);
                }
            }
        }
        all.into_iter()
            .filter_map(|(l, v)| (v.len() == 1).then(|| (l, v[0])))
            .collect()
    };
    let small = |a: Local| {
        promised.contains(&a)
            || matches!(single_defs.get(&a), Some(Rvalue::BinaryOp(BinOp::Div | BinOp::Rem, x, d))
                if local(x).is_some_and(|x| promised.contains(&x)) && int(d).is_some_and(|d| d >= 1))
    };
    // the setup of a loop in the block before it: `end = Add(copy a, const K)` and `v = copy a`, a promised; its a
    // and K, and the two lines
    let setup = |b: &ir::Block, l: &Counted| -> Option<(Local, i128, [usize; 2])> {
        let mut e = None;
        let mut v = None;
        for (k, s) in b.statements.iter().enumerate() {
            match s {
                Statement::Assign(d, Rvalue::BinaryOp(BinOp::Add, a, n)) if d.as_local() == Some(l.end) => {
                    e = Some((local(a)?, int(n)?, k));
                }
                Statement::Assign(d, Rvalue::Use(a)) if d.as_local() == Some(l.v) => v = Some((local(a)?, k)),
                _ => {}
            }
        }
        let ((a, n, ke), (a2, kv)) = (e?, v?);
        (a == a2 && small(a) && (1..=10).contains(&n)).then_some((a, n, [ke, kv]))
    };
    for h1 in 0..blocks.len() {
        let Some(outer) = counted(h1) else { continue };
        let outer_name = blocks[h1].name;
        let ins = preds.get(&outer_name).cloned().unwrap_or_default();
        // one loop or two: the inner one set up in the outer one's body, its exit the outer one's latch
        let b1 = &blocks[outer.body];
        let inner = match &b1.terminator {
            Terminator::Goto(h2) => at.get(h2).and_then(|&h2| counted(h2)),
            _ => None,
        };
        let (loops, sum_block, latches): (Vec<(&Counted, usize)>, usize, Vec<usize>) = match &inner {
            Some(inner) => {
                let Some(&l1) = at.get(&inner.exit) else { continue };
                let latch = &blocks[l1];
                let one_more = |b: &ir::Block, v: Local| {
                    matches!(b.statements.as_slice(), [Statement::Assign(d, Rvalue::BinaryOp(BinOp::Add, x, k))]
                        if d.as_local() == Some(v) && local(x) == Some(v) && int(k) == Some(1))
                };
                if !one_more(latch, outer.v) || latch.terminator != Terminator::Goto(outer_name) {
                    continue;
                }
                if b1.statements.len() != 2 {
                    continue;
                }
                (vec![(&outer, h1), (inner, outer.body)], inner.body, vec![l1])
            }
            None => (vec![(&outer, h1)], outer.body, vec![]),
        };
        // the sum checked for overflow (`t = AddWithOverflow(acc, ..)`, its assert, then `acc = move (t.0)` and the
        // variable one more in a block of their own): that block, the pair and the sum's local
        let innermost_name = blocks[loops.last().unwrap().0.header].name;
        let checked: Option<(usize, Local, Local)> = match &blocks[sum_block].terminator {
            Terminator::Goto(h) if *h == innermost_name => None,
            Terminator::Assert {
                cond: Operand::Copy(p) | Operand::Move(p),
                expected: false,
                target,
                ..
            } => {
                let (Some(pair), [Projection::Field(1, _)]) = (p.base_local(), p.projection.as_slice()) else {
                    continue;
                };
                let Some(&c) = at.get(target) else { continue };
                let cb = &blocks[c];
                let ok_preds = preds.get(target).map(Vec::as_slice) == Some(&[blocks[sum_block].name][..]);
                let Some(Statement::Assign(d, Rvalue::Use(Operand::Copy(v) | Operand::Move(v)))) =
                    cb.statements.first()
                else {
                    continue;
                };
                let (Some(acc), Some(from), [Projection::Field(0, _)]) =
                    (d.as_local(), v.base_local(), v.projection.as_slice())
                else {
                    continue;
                };
                if !ok_preds
                    || from != pair
                    || cb.statements.len() != 2
                    || cb.terminator != Terminator::Goto(innermost_name)
                {
                    continue;
                }
                Some((c, pair, acc))
            }
            _ => continue,
        };
        // the block that goes back to the innermost header, the variable one more on it
        let tail = checked.map_or(sum_block, |(c, _, _)| c);
        // the block before the outer loop (the way in that isn't the way back round), and each loop's start and count
        let back_name = blocks[latches.first().copied().unwrap_or(tail)].name;
        let pre_name = match ins.as_slice() {
            [x, y] if *y == back_name => *x,
            [x, y] if *x == back_name => *y,
            _ => continue,
        };
        let Some(&pre) = at.get(&pre_name) else { continue };
        if blocks[pre].terminator != Terminator::Goto(outer_name) {
            continue;
        }
        let mut starts = vec![];
        let mut setup_lines: Vec<(usize, usize)> = vec![];
        let mut fine = true;
        for (k, (l, _)) in loops.iter().enumerate() {
            let where_set = if k == 0 { pre } else { outer.body };
            match setup(&blocks[where_set], l) {
                Some((a, n, lines)) => {
                    starts.push((a, n));
                    setup_lines.extend(lines.iter().map(|x| (where_set, *x)));
                }
                None => fine = false,
            }
        }
        if !fine {
            continue;
        }
        // the sum's block: the index worked out, the sum, the innermost variable one more, and back to its header
        let innermost = loops.last().unwrap().0;
        let Some(last) = blocks[tail].statements.last() else {
            continue;
        };
        if !matches!(last, Statement::Assign(d, Rvalue::BinaryOp(BinOp::Add, x, k))
            if d.as_local() == Some(innermost.v) && local(x) == Some(innermost.v) && int(k) == Some(1))
        {
            continue;
        }
        let body = &blocks[sum_block];
        let rest: &[Statement] = if checked.is_some() {
            &body.statements
        } else {
            &body.statements[..body.statements.len() - 1]
        };
        let vars: Vec<Local> = loops.iter().map(|(l, _)| l.v).collect();
        let mut forms: HashMap<Local, Affine> = HashMap::new();
        let form_of = |forms: &HashMap<Local, Affine>, o: &Operand| -> Option<Affine> {
            if let Some(v) = int(o) {
                return Some((v, Default::default()));
            }
            let l = local(o)?;
            Some(
                forms
                    .get(&l)
                    .cloned()
                    .unwrap_or_else(|| (0, [(l, 1)].into_iter().collect())),
            )
        };
        let add = |a: Affine, b: Affine| {
            let mut m = a.1;
            for (l, c) in b.1 {
                *m.entry(l).or_default() += c;
            }
            (a.0 + b.0, m)
        };
        let mut sum: Option<(Local, Local, Affine)> = None;
        // an element read into a local of its own first, `_22 = copy A[_45]`, as with overflow checks on
        let mut element_reads: HashMap<Local, Operand> = HashMap::new();
        let element_read = |o: &Operand| match o {
            Operand::Copy(p) | Operand::Move(p) => {
                matches!(
                    p.projection.as_slice(),
                    [Projection::Index(_) | Projection::IndexPlus(..)]
                )
            }
            _ => false,
        };
        for s in rest {
            let Statement::Assign(d, rv) = s else {
                fine = false;
                break;
            };
            let Some(t) = d.as_local() else {
                fine = false;
                break;
            };
            if let Rvalue::Use(o) = rv {
                if element_read(o) && sum.is_none() {
                    element_reads.insert(t, o.clone());
                    continue;
                }
            }
            let f = match rv {
                Rvalue::Use(o) => form_of(&forms, o),
                Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::AddWithOverflow), x, y) => {
                    let y = &local(y)
                        .and_then(|l| element_reads.get(&l).cloned())
                        .unwrap_or_else(|| y.clone());
                    // the sum: `acc = Add(copy acc, copy A[index])`, or checked into a pair whose value is acc's
                    let read = |o: &Operand| match o {
                        Operand::Copy(p) | Operand::Move(p) => match (p.base_local(), p.projection.as_slice()) {
                            (Some(a), [Projection::Index(x)]) => Some((a, *x, 0)),
                            (Some(a), [Projection::IndexPlus(x, k)]) => Some((a, *x, *k as i128)),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let (Some(acc), Some((a, x, k))) = (local(x), read(y)) {
                        let into = match checked {
                            Some((_, pair, acc)) if t == pair && *op == BinOp::AddWithOverflow => acc,
                            _ if *op == BinOp::Add => t,
                            _ => Local(u32::MAX),
                        };
                        if acc == into && sum.is_none() {
                            let index = add(
                                form_of(&forms, &Operand::Copy(Place::local(x))).unwrap(),
                                (k, Default::default()),
                            );
                            sum = Some((acc, a, index));
                            continue;
                        }
                    }
                    match (form_of(&forms, x), form_of(&forms, y)) {
                        (Some(a), Some(b)) if local(x).is_some() || local(y).is_some() => Some(add(a, b)),
                        _ => None,
                    }
                }
                Rvalue::BinaryOp(BinOp::Mul, x, y) => match (int(x), int(y)) {
                    (Some(k), None) | (None, Some(k)) => {
                        let o = if int(x).is_some() { y } else { x };
                        form_of(&forms, o).map(|(c, m)| (c * k, m.into_iter().map(|(l, v)| (l, v * k)).collect()))
                    }
                    _ => None,
                },
                _ => None,
            };
            let Some(f) = f else {
                fine = false;
                break;
            };
            forms.insert(t, f);
        }
        let Some((acc, array, index)) = sum.filter(|_| fine) else {
            continue;
        };
        // the nest's blocks, and what they name: the variables, temps, ends and comparisons, read nowhere else
        let mut nest: Vec<usize> = loops.iter().map(|(l, _)| l.header).collect();
        nest.extend(latches.iter().copied());
        nest.push(sum_block);
        nest.push(tail);
        if inner.is_some() {
            nest.push(outer.body);
        }
        nest.sort_unstable();
        nest.dedup();
        let mut own: Vec<Local> = forms.keys().copied().collect();
        own.extend(checked.map(|(_, pair, _)| pair));
        own.extend(element_reads.keys().copied());
        own.extend(loops.iter().flat_map(|(l, _)| [l.v, l.end, l.cmp]));
        let in_nest = |l: Local| -> usize {
            nest.iter()
                .map(|&i| {
                    blocks[i].statements.iter().map(|s| s.count(l)).sum::<usize>() + blocks[i].terminator.count(l)
                })
                .sum::<usize>()
                + setup_lines
                    .iter()
                    .filter(|(b, _)| !nest.contains(b))
                    .map(|&(b, k)| blocks[b].statements[k].count(l))
                    .sum::<usize>()
        };
        let everywhere = |l: Local| -> usize {
            blocks
                .iter()
                .map(|b| b.statements.iter().map(|s| s.count(l)).sum::<usize>() + b.terminator.count(l))
                .sum()
        };
        // (the sum read twice in the nest, as the sum and set from it; and as the checked sum's assert's message
        // says it)
        let in_message = if checked.is_some() {
            blocks[sum_block].terminator.count(acc)
        } else {
            0
        };
        if own.iter().any(|&l| in_nest(l) != everywhere(l)) || in_nest(acc) != 2 + in_message {
            continue;
        }
        // nothing comes into the nest but at the outer loop's header, from the block before it
        let names: HashSet<BasicBlock> = nest.iter().map(|&i| blocks[i].name).collect();
        let entered = nest.iter().all(|&i| {
            preds
                .get(&blocks[i].name)
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .all(|p| names.contains(p) || (i == h1 && *p == pre_name))
        });
        if !entered {
            continue;
        }
        // the array, the sum and the starts set nowhere in the nest
        let set_in_nest = |l: Local| {
            nest.iter().any(|&i| {
                blocks[i]
                    .statements
                    .iter()
                    .any(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(l)))
            })
        };
        let invariant = index
            .1
            .keys()
            .filter(|l| !vars.contains(l))
            .any(|l| set_in_nest(*l) || *l == acc);
        if invariant
            || set_in_nest(array)
            || starts.iter().any(|(a, _)| set_in_nest(*a))
            || !matches!(locals.get(&array), Some(Ty::Array(..)))
        {
            continue;
        }
        // the index with each variable v = start + step: base (the rest, and each start times v's coefficient) and
        // the offset of each step, row after row
        let (konst, coefs) = index.clone();
        let mut base: std::collections::BTreeMap<Local, i128> = std::collections::BTreeMap::new();
        let mut steps: Vec<i128> = vec![];
        for (l, c) in &coefs {
            if !vars.contains(l) {
                base.insert(*l, *c);
            }
        }
        for (k, v) in vars.iter().enumerate() {
            let c = coefs.get(v).copied().unwrap_or(0);
            *base.entry(starts[k].0).or_default() += c;
            steps.push(c);
        }
        if konst < 0 || steps.iter().any(|c| *c < 0) || base.iter().any(|(l, c)| *c < 0 || (*c != 0 && !small(*l))) {
            continue;
        }
        let mut offsets: Vec<i128> = vec![konst];
        for (k, step) in steps.iter().enumerate() {
            let n = starts[k].1;
            offsets = offsets.iter().flat_map(|o| (0..n).map(move |p| o + p * step)).collect();
        }
        if offsets.len() < 2 || offsets.iter().any(|o| *o > 999) {
            continue;
        }
        // the middle square taken away just after (`n - b[r + 1][c + 1]`), from an array whose sum over the window
        // can't pass a card: the same as leaving it out of the table, so it is, and the subtraction is a copy
        let mut taken_out: Option<(usize, usize)> = None;
        let tables = super::ranges::constant_tables(blocks, &super::ranges::taken_locals(blocks));
        let bounded = tables
            .get(&array)
            .is_some_and(|r| r[0].abs().max(r[1].abs()).saturating_mul(offsets.len() as i64) <= 999);
        // a checked sum stays a card from where it starts, a constant set just before the loops: then it can't
        // overflow, and the loop over the table adds plainly
        if checked.is_some() {
            let start = blocks[pre].statements.iter().rev().find(|s| s.count(acc) > 0);
            let c0 = match start {
                Some(Statement::Assign(d, Rvalue::Use(Operand::Constant(Const::Int { value, .. }))))
                    if d.as_local() == Some(acc) =>
                {
                    Some(*value as i64)
                }
                _ => None,
            };
            let fits = c0.zip(tables.get(&array)).is_some_and(|(c0, r)| {
                c0.abs() + r[0].abs().max(r[1].abs()).saturating_mul(offsets.len() as i64) <= 999
            });
            if !fits {
                continue;
            }
        }
        if let (Some(&xi), true) = (at.get(&outer.exit), bounded) {
            let mut later: HashMap<Local, Affine> = HashMap::new();
            // an element read into a local first, `_28 = copy b[_47+11]`: the read
            let mut later_reads: HashMap<Local, (Local, i128)> = HashMap::new();
            let read_of = |o: &Operand| match o {
                Operand::Copy(p) | Operand::Move(p) => match (p.base_local(), p.projection.as_slice()) {
                    (Some(a), [Projection::Index(x)]) if a == array => Some((*x, 0)),
                    (Some(a), [Projection::IndexPlus(x, k)]) if a == array => Some((*x, *k as i128)),
                    _ => None,
                },
                _ => None,
            };
            for (si, s) in blocks[xi].statements.iter().enumerate() {
                let Statement::Assign(d, rv) = s else { break };
                let Some(t) = d.as_local() else { break };
                if let (Rvalue::Use(o), false) = (rv, s.count(acc) > 0) {
                    if let Some(r) = read_of(o) {
                        later_reads.insert(t, r);
                        continue;
                    }
                }
                if s.count(acc) > 0 {
                    let checked_sub = matches!(rv, Rvalue::BinaryOp(BinOp::SubWithOverflow, ..));
                    // (checked, only as the block's last line, its assert the block's end)
                    let last_line = si + 1 == blocks[xi].statements.len()
                        && matches!(&blocks[xi].terminator, Terminator::Assert { cond: Operand::Copy(p) | Operand::Move(p), expected: false, .. }
                            if p.base_local() == Some(t) && matches!(p.projection.as_slice(), [Projection::Field(1, _)]));
                    if let Rvalue::BinaryOp(BinOp::Sub | BinOp::SubWithOverflow, x, y) = rv {
                        if checked_sub && !last_line {
                            break;
                        }
                        // (a local read into once, and read only here)
                        let read = read_of(y).or_else(|| {
                            let l = local(y)?;
                            let total: usize = blocks
                                .iter()
                                .map(|b| b.statements.iter().map(|s| s.count(l)).sum::<usize>() + b.terminator.count(l))
                                .sum();
                            let in_message = blocks[xi].terminator.count(l);
                            (total == 2 + in_message).then_some(())?;
                            later_reads.get(&l).copied()
                        });
                        if let (Some(a0), Some((ix, k))) = (local(x), read) {
                            let f = later.get(&ix).cloned().unwrap_or((0, [(ix, 1)].into_iter().collect()));
                            let mut want = base.clone();
                            want.retain(|_, c| *c != 0);
                            let mut got = f.1.clone();
                            got.retain(|_, c| *c != 0);
                            let off = f.0 + k;
                            if a0 == acc && got == want {
                                if let Some(pos) = offsets.iter().position(|o| *o == off) {
                                    taken_out = Some((si, pos));
                                }
                            }
                        }
                    }
                    break;
                }
                let f = match rv {
                    Rvalue::BinaryOp(BinOp::Mul, x, y) => match (int(x), int(y), local(x), local(y)) {
                        (None, Some(k), Some(l), _) | (Some(k), None, _, Some(l)) => {
                            let (c, m) = later.get(&l).cloned().unwrap_or((0, [(l, 1)].into_iter().collect()));
                            Some((c * k, m.into_iter().map(|(l, v)| (l, v * k)).collect()))
                        }
                        _ => None,
                    },
                    Rvalue::BinaryOp(BinOp::Add, x, y) => {
                        let f = |o: &Operand| {
                            int(o).map(|v| (v, Default::default())).or_else(|| {
                                let l = local(o)?;
                                Some(later.get(&l).cloned().unwrap_or((0, [(l, 1)].into_iter().collect())))
                            })
                        };
                        match (f(x), f(y)) {
                            (Some(a), Some(b)) => Some(add(a, b)),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(f) = f {
                    later.insert(t, f);
                }
            }
        }
        if let Some((si, pos)) = taken_out {
            let xi = at[&outer.exit];
            let ty = locals.get(&acc).cloned().unwrap_or(Ty::Int(IntTy::I16));
            if let Statement::Assign(d, rv) = &mut blocks[xi].statements[si] {
                if matches!(rv, Rvalue::BinaryOp(BinOp::SubWithOverflow, ..)) {
                    // the pair's value, its check gone: it can't overflow
                    *d = Place {
                        base: d.base.clone(),
                        projection: vec![Projection::Field(0, ty)],
                    };
                    if let Terminator::Assert { target, .. } = blocks[xi].terminator {
                        blocks[xi].terminator = Terminator::Goto(target);
                    }
                }
            }
            if let Statement::Assign(_, rv) = &mut blocks[xi].statements[si] {
                *rv = Rvalue::Use(Operand::Copy(Place::local(acc)));
            }
            offsets.remove(pos);
        }
        // the nest replaced: base worked out where the loops were set up, then the loop over the table
        let usize_ty = Ty::Int(IntTy::Usize);
        let mut next_local = locals.keys().map(|l| l.0).max().unwrap_or(0) + 1;
        let mut fresh = |locals: &mut crate::compile::util::Ordered<Local, Ty>, ty: Ty| {
            let l = Local(next_local);
            next_local += 1;
            locals.set(l, ty);
            l
        };
        let k = |v: i128| {
            Operand::Constant(Const::Int {
                value: v,
                ty: IntTy::Usize,
            })
        };
        let copy = |l: Local| Operand::Copy(Place::local(l));
        let mut lines: Vec<Statement> = vec![];
        let mut total: Option<Local> = None;
        for (l, c) in base.iter().filter(|(_, c)| **c != 0) {
            let term = if *c == 1 {
                *l
            } else {
                let t = fresh(locals, usize_ty.clone());
                lines.push(Statement::Assign(
                    Place::local(t),
                    Rvalue::BinaryOp(BinOp::Mul, copy(*l), k(*c)),
                ));
                t
            };
            total = Some(match total {
                None => term,
                Some(sofar) => {
                    let t = fresh(locals, usize_ty.clone());
                    lines.push(Statement::Assign(
                        Place::local(t),
                        Rvalue::BinaryOp(BinOp::Add, copy(sofar), copy(term)),
                    ));
                    t
                }
            });
        }
        let base_local = match total {
            Some(t) => t,
            None => {
                let t = fresh(locals, usize_ty.clone());
                lines.push(Statement::Assign(Place::local(t), Rvalue::Use(k(0))));
                t
            }
        };
        let n = offsets.len();
        let table = fresh(locals, Ty::Array(Box::new(usize_ty.clone()), ArrayLen::Known(n as u64)));
        let q = fresh(locals, usize_ty.clone());
        let (w, idx) = (fresh(locals, usize_ty.clone()), fresh(locals, usize_ty.clone()));
        let elem = match locals.get(&array) {
            Some(Ty::Array(e, _)) => (**e).clone(),
            _ => unreachable!(),
        };
        let e = fresh(locals, elem);
        let more = fresh(locals, Ty::Bool);
        lines.push(Statement::Assign(
            Place::local(table),
            Rvalue::Array(offsets.iter().map(|o| k(*o)).collect()),
        ));
        lines.push(Statement::Assign(Place::local(q), Rvalue::Use(k(0))));
        let looped_name = BasicBlock(blocks.iter().map(|b| b.name.0).max().unwrap_or(0).max(860_000) + 1);
        let at_ = |a: Local, i: Local| Place {
            base: PlaceBase::Local(a),
            projection: vec![Projection::Index(i)],
        };
        let looped = ir::Block {
            name: looped_name,
            statements: vec![
                Statement::Assign(Place::local(w), Rvalue::Use(Operand::Copy(at_(table, q)))),
                Statement::Assign(
                    Place::local(idx),
                    Rvalue::BinaryOp(BinOp::Add, copy(base_local), copy(w)),
                ),
                Statement::Assign(Place::local(e), Rvalue::Use(Operand::Copy(at_(array, idx)))),
                Statement::Assign(Place::local(acc), Rvalue::BinaryOp(BinOp::Add, copy(acc), copy(e))),
                Statement::Assign(Place::local(q), Rvalue::BinaryOp(BinOp::Add, copy(q), k(1))),
                Statement::Assign(Place::local(more), Rvalue::BinaryOp(BinOp::Lt, copy(q), k(n as i128))),
            ],
            terminator: Terminator::SwitchInt {
                discr: Operand::Move(Place::local(more)),
                arms: vec![(0, outer.exit)],
                otherwise: Some(looped_name),
            },
        };
        // (the setup's lines go: their locals are the nest's own)
        let mut drop_lines: Vec<(usize, usize)> = setup_lines.clone();
        drop_lines.sort_unstable();
        for &(b, l) in drop_lines.iter().rev() {
            if b == pre {
                blocks[b].statements.remove(l);
            }
        }
        blocks[pre].statements.extend(lines);
        blocks[pre].terminator = Terminator::Goto(looped_name);
        let gone: HashSet<BasicBlock> = nest.iter().map(|&i| blocks[i].name).collect();
        blocks.retain(|b| !gone.contains(&b.name));
        blocks.push(looped);
        return true;
    }
    false
}

/// A number split into its digits and put back together: `t = (s / d) * d + s % d` is s, as Rust's division and
/// remainder are defined (for any integer s and d not 0, `(s / d) * d + s % d == s`), and the product can't pass
/// what s is. So a grid's index `10 * r + c`, with r and c the digits of the square read as one number, is that
/// number. Each part set once, s too, so each is what it was made from wherever it is read.
pub fn digits_rejoined(blocks: &mut [ir::Block]) {
    use ir::{BinOp, Local, Operand, Place};
    let local = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    let int = |o: &Operand| match o {
        Operand::Constant(ir::Const::Int { value, .. }) => Some(*value),
        _ => None,
    };
    let mut all: HashMap<Local, Vec<Rvalue>> = HashMap::new();
    for b in blocks.iter() {
        for st in &b.statements {
            if let Statement::Assign(d, rv) = st {
                all.entry(d.base_local().unwrap()).or_default().push(rv.clone());
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            all.entry(dest.base_local().unwrap())
                .or_default()
                .push(Rvalue::Use(Operand::Copy(Place::local(Local(u32::MAX)))));
        }
    }
    let once = |l: Local| all.get(&l).and_then(|v| (v.len() == 1).then(|| v[0].clone()));
    // (s / d) * d, as `Mul(copy q, const d)` with `q = Div(copy s, const d)`: s and d
    let times = |o: &Operand| -> Option<(Local, i128)> {
        let Rvalue::BinaryOp(BinOp::Mul, x, y) = once(local(o)?)? else {
            return None;
        };
        let (q, d) = match (local(&x), int(&y), int(&x), local(&y)) {
            (Some(q), Some(d), _, _) | (_, _, Some(d), Some(q)) => (q, d),
            _ => return None,
        };
        let Rvalue::BinaryOp(BinOp::Div, s, d2) = once(q)? else {
            return None;
        };
        (int(&d2) == Some(d) && d != 0).then_some(())?;
        Some((local(&s)?, d))
    };
    let rem = |o: &Operand| -> Option<(Local, i128)> {
        let Rvalue::BinaryOp(BinOp::Rem, s, d) = once(local(o)?)? else {
            return None;
        };
        Some((local(&s)?, int(&d)?))
    };
    for b in blocks.iter_mut() {
        for st in b.statements.iter_mut() {
            let Statement::Assign(_, rv @ Rvalue::BinaryOp(BinOp::Add, _, _)) = st else {
                continue;
            };
            let Rvalue::BinaryOp(_, x, y) = &*rv else { continue };
            let joined = match (times(x).zip(rem(y)), times(y).zip(rem(x))) {
                (Some((a, b)), _) | (_, Some((a, b))) if a == b && once(a.0).is_some() => Some(a.0),
                _ => None,
            };
            if let Some(s) = joined {
                *rv = Rvalue::Use(Operand::Copy(Place::local(s)));
            }
        }
    }
}

/// A comparison, whose value is a bool.
fn is_comparison(op: ir::BinOp) -> bool {
    use ir::BinOp::*;
    matches!(op, Lt | Le | Gt | Ge | Eq | Ne)
}

/// A check made again: an assert where the only way in comes straight from a block that asserted the same thing,
/// expecting the same, with nothing it reads set in between. The same thing is the same bool (`assert(copy _20)`,
/// as GVN leaves one for each `b[r][..]` that indexes by the same r), or a bool each block works out from the same
/// comparison of the same locals (`_33 < 6` again for `b[r + 1][c]` and `b[r + 1][c + 2]`). Once the first holds,
/// the second does: it is a goto.
pub fn repeated_checks(blocks: &mut [ir::Block]) {
    let preds = predecessors(blocks);
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let borrowed: HashSet<ir::Local> = blocks
        .iter()
        .flat_map(|b| &b.statements)
        .filter_map(|s| match s {
            Statement::Assign(_, ir::Rvalue::Ref(_, p)) => p.base_local(),
            _ => None,
        })
        .collect();
    let sets = |s: &Statement, xs: &[ir::Local]| matches!(s, Statement::Assign(d, _) if d.base_local().is_some_and(|v| xs.contains(&v)));
    // what a block's assert asserts, the locals that must keep their values, and the line it is worked out on
    // (none: the bool comes from before the block)
    let asserted = |b: &ir::Block| -> Option<(Rvalue, bool, Vec<ir::Local>, Option<usize>)> {
        let Terminator::Assert {
            cond: ir::Operand::Copy(p) | ir::Operand::Move(p),
            expected,
            ..
        } = &b.terminator
        else {
            return None;
        };
        // an overflow check's flag, `!copy (_34.1: bool)`, asserted again as GVN leaves it: the same place
        if let (Some(pair), [ir::Projection::Field(1, _)]) = (p.base_local(), p.projection.as_slice()) {
            if borrowed.contains(&pair) {
                return None;
            }
            let def = b
                .statements
                .iter()
                .rposition(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(pair)));
            let flag = Rvalue::Use(ir::Operand::Copy(p.clone()));
            return Some((flag, *expected, vec![pair], def));
        }
        let v = p.as_local().filter(|v| !borrowed.contains(v))?;
        let def = b
            .statements
            .iter()
            .rposition(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(v)));
        let Some(k) = def else {
            let bool_itself = Rvalue::Use(ir::Operand::Copy(ir::Place::local(v)));
            return Some((bool_itself, *expected, vec![v], None));
        };
        let Statement::Assign(d, Rvalue::BinaryOp(op, x, y)) = &b.statements[k] else {
            return None;
        };
        let plain = |o: &ir::Operand| match o {
            ir::Operand::Constant(_) => Some(None),
            ir::Operand::Copy(q) | ir::Operand::Move(q) => q.as_local().filter(|l| !borrowed.contains(l)).map(Some),
            _ => None,
        };
        if d.as_local() != Some(v) || !is_comparison(*op) {
            return None;
        }
        let reads: Vec<ir::Local> = [plain(x)?, plain(y)?].into_iter().flatten().collect();
        let same = |o: &ir::Operand| match o {
            ir::Operand::Move(q) => ir::Operand::Copy(q.clone()),
            o => o.clone(),
        };
        Some((Rvalue::BinaryOp(*op, same(x), same(y)), *expected, reads, Some(k)))
    };
    let mut settled = vec![];
    for (i, b) in blocks.iter().enumerate() {
        let Terminator::Assert { target, .. } = &b.terminator else {
            continue;
        };
        let Some((what, expected, reads, def)) = asserted(b) else {
            continue;
        };
        // (a flag worked out again in this block is a check of its own)
        if matches!(what, Rvalue::Use(_)) && def.is_some() {
            continue;
        }
        // nothing it reads set on this block's lines before the assert's own
        let before = def.unwrap_or(b.statements.len());
        if b.statements[..before].iter().any(|s| sets(s, &reads)) {
            continue;
        }
        let mut cur = i;
        // (a bounded walk back along blocks with one way in)
        for _ in 0..64 {
            let ins = preds.get(&blocks[cur].name).map(Vec::as_slice).unwrap_or_default();
            let ([p], true) = (ins, cur != 0) else { break };
            let Some(&j) = at.get(p) else { break };
            let earlier = &blocks[j];
            if let (Terminator::Assert { target: t, .. }, Some((w, e, _, d))) = (&earlier.terminator, asserted(earlier))
            {
                let after = d.map_or(0, |k| k + 1);
                if w == what
                    && e == expected
                    && *t == blocks[cur].name
                    && !earlier.statements[after..].iter().any(|s| sets(s, &reads))
                {
                    settled.push((i, *target));
                    break;
                }
            }
            if earlier.statements.iter().any(|s| sets(s, &reads))
                || matches!(&earlier.terminator, Terminator::Call { dest, .. } if dest.base_local().is_some_and(|v| reads.contains(&v)))
                || earlier.terminator.successors().len() != 1
            {
                break;
            }
            cur = j;
        }
    }
    for (i, target) in settled {
        blocks[i].terminator = Terminator::Goto(target);
    }
}

/// Checks rustc has already worked out, `assert(const false, ..)` or `assert(!const true, ..)` (a program that
/// `#[allow(unconditional_panic)]`): one that holds is a goto, and one that fails is the panic, every time. Nothing the
/// failing block did before it can be seen once the program has panicked, so it keeps no statements, and it goes on
/// to itself (it never does): what came after it is reached from nowhere, and goes, with the statements that only
/// fed it, such as an index of `-1 as usize` (`usize::MAX`, past a card) that only the failing bounds check read, or
/// an input only Rust's message would have printed.
pub fn settled_checks(blocks: &mut Vec<ir::Block>, results: &HashSet<ir::Local>) {
    let mut failed = false;
    for b in blocks.iter_mut() {
        let Terminator::Assert {
            cond: ir::Operand::Constant(ir::Const::Bool(holds)),
            expected,
            msg_args,
            target,
            unwind: ir::Unwind::Continue | ir::Unwind::Unreachable,
            ..
        } = &mut b.terminator
        else {
            continue;
        };
        if *holds == *expected {
            b.terminator = Terminator::Goto(*target);
        } else {
            // the panic says why without the values Rust's message prints
            msg_args.clear();
            *target = b.name;
            b.statements.clear();
            failed = true;
        }
    }
    if !failed {
        return;
    }
    let mut reached = HashSet::from([BasicBlock(0)]);
    let mut work = vec![BasicBlock(0)];
    while let Some(n) = work.pop() {
        if let Some(b) = blocks.iter().find(|b| b.name == n) {
            for s in b.terminator.successors() {
                if reached.insert(s) {
                    work.push(s);
                }
            }
        }
    }
    blocks.retain(|b| reached.contains(&b.name));
    dead_code(blocks, results);
}

/// A value set to a constant and at once tested: a block whose last write of x is a constant, which jumps to a block
/// that does nothing but test x (`switchInt(x)`, or `d = x op k; switchInt(d)`), jumps to the arm the constant picks
/// instead, setting d itself. `if c { 1 } else { 0 }` tested again, an enum's tag set and then matched (the
/// methods inlined at opt-level 1): on CARDIAC each test is a subtraction and a TAC or two.
pub fn thread_constants(blocks: &mut Vec<ir::Block>) {
    use ir::{BinOp, Const, Operand, Place};
    let int = |c: &Const| match c {
        Const::Int { value, .. } => Some(*value),
        Const::Bool(b) => Some(*b as i128),
        _ => None,
    };
    let local = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    /// How a tested value follows from x: x op k (or k op x, flipped).
    type Derived = Option<(BinOp, i128, bool)>;
    // each testing block: its name, the local it reads, how the tested value follows from it, and the switch
    struct Test {
        name: BasicBlock,
        x: Local,
        /// d, and the op and constant (d = x op k), or none: the switch reads x itself
        derived: Option<(Local, Derived)>,
        arms: Vec<(i128, BasicBlock)>,
        otherwise: Option<BasicBlock>,
    }
    let mut tests = vec![];
    for b in blocks.iter() {
        let Terminator::SwitchInt { discr, arms, otherwise } = &b.terminator else {
            continue;
        };
        let Some(s) = local(discr) else { continue };
        let (x, derived) = match b.statements.as_slice() {
            [] => (s, None),
            [Statement::Assign(d, rv)] if d.as_local() == Some(s) => match rv {
                Rvalue::Use(o) => match local(o) {
                    Some(x) => (x, Some((s, None))),
                    None => continue,
                },
                Rvalue::BinaryOp(
                    op @ (BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::Add
                    | BinOp::Sub),
                    a,
                    c,
                ) => match (local(a), c, local(c), a) {
                    (Some(x), Operand::Constant(k), _, _) => match int(k) {
                        Some(k) => (x, Some((s, Some((*op, k, false))))),
                        None => continue,
                    },
                    (_, _, Some(x), Operand::Constant(k)) => match int(k) {
                        Some(k) => (x, Some((s, Some((*op, k, true))))),
                        None => continue,
                    },
                    _ => continue,
                },
                _ => continue,
            },
            _ => continue,
        };
        tests.push(Test {
            name: b.name,
            x,
            derived,
            arms: arms.clone(),
            otherwise: *otherwise,
        });
    }
    // a local whose address is taken may be written through it: not one to follow
    let mut taken = HashSet::new();
    for b in blocks.iter() {
        for s in &b.statements {
            if let Statement::Assign(_, Rvalue::Ref(_, p)) = s {
                taken.extend(p.base_local());
            }
        }
    }
    tests.retain(|t| !taken.contains(&t.x));
    let mut threaded = false;
    for t in &tests {
        for b in blocks.iter_mut() {
            if b.terminator != Terminator::Goto(t.name) || b.name == t.name {
                continue;
            }
            // x's last write in the block: a constant
            let last = b
                .statements
                .iter()
                .rev()
                .find(|s| matches!(s, Statement::Assign(p, _) if p.base_local() == Some(t.x)));
            let Some(Statement::Assign(p, _)) = last else { continue };
            if p.as_local() != Some(t.x) {
                continue;
            }
            let Some(Statement::Assign(_, Rvalue::Use(Operand::Constant(c)))) = last else {
                continue;
            };
            let Some(v) = int(c) else { continue };
            let (tested, set) = match t.derived {
                None => (v, None),
                Some((d, None)) => (
                    v,
                    Some(Statement::Assign(
                        Place::local(d),
                        Rvalue::Use(Operand::Constant(c.clone())),
                    )),
                ),
                Some((d, Some((op, k, flipped)))) => {
                    let (l, r) = if flipped { (k, v) } else { (v, k) };
                    let constant =
                        |c: Const| Some(Statement::Assign(Place::local(d), Rvalue::Use(Operand::Constant(c))));
                    match op {
                        // a number, x + k or x - k (a sign tag's discriminant)
                        BinOp::Add | BinOp::Sub => {
                            let n = if op == BinOp::Add { l + r } else { l - r };
                            let ty = match c {
                                Const::Int { ty, .. } => *ty,
                                _ => ir::IntTy::I16,
                            };
                            (n, constant(Const::Int { value: n, ty }))
                        }
                        BinOp::Eq => ((l == r) as i128, constant(Const::Bool(l == r))),
                        BinOp::Ne => ((l != r) as i128, constant(Const::Bool(l != r))),
                        BinOp::Lt => ((l < r) as i128, constant(Const::Bool(l < r))),
                        BinOp::Le => ((l <= r) as i128, constant(Const::Bool(l <= r))),
                        BinOp::Gt => ((l > r) as i128, constant(Const::Bool(l > r))),
                        _ => ((l >= r) as i128, constant(Const::Bool(l >= r))),
                    }
                }
            };
            let Some(to) = t
                .arms
                .iter()
                .find(|(a, _)| *a == tested)
                .map(|(_, to)| *to)
                .or(t.otherwise)
            else {
                continue;
            };
            b.statements.extend(set);
            b.terminator = Terminator::Goto(to);
            threaded = true;
        }
    }
    // what the entry no longer reaches goes (a loop jumped past is a cycle of blocks reaching only each other)
    if threaded {
        let at: HashMap<BasicBlock, &Block> = blocks.iter().map(|b| (b.name, b)).collect();
        let mut seen = HashSet::new();
        let mut todo: Vec<BasicBlock> = blocks.first().map(|b| b.name).into_iter().collect();
        while let Some(n) = todo.pop() {
            if seen.insert(n) {
                todo.extend(at.get(&n).into_iter().flat_map(|b| b.terminator.successors()));
            }
        }
        blocks.retain(|b| seen.contains(&b.name));
    }
}

/// A counter that only counts: a local set to constants, stepped by a constant, and compared with one constant k
/// and nothing else (`for _ in 0..96`, whose value nothing reads, or `while n < 96 { ..; n += 1 }` where nothing else reads n).
/// It counts to 0 instead, from each constant minus k: CARDIAC's one test is "below 0", so `c < 0` is a load and a
/// TAC, where `c < k` is a subtraction more and k in a cell of its own. (A named variable too: its value is no one's
/// to read but the loop's.) Parameters and the result (`_0`, and a struct result's cells) are left as they are.
/// Returns the counters rebased: each is an i16 now, whatever its type was.
pub fn rebase_counters(
    blocks: &mut [Block],
    types: &HashMap<Local, ir::Ty>,
    keep: &HashSet<Local>,
    params: &HashSet<Local>,
) -> Vec<Local> {
    use ir::{BinOp, Const, IntTy, Operand};
    let int = |o: &Operand| match o {
        Operand::Constant(Const::Int { value, .. }) => Some(*value),
        _ => None,
    };
    let local = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    let comparison = |op: &BinOp| {
        matches!(
            op,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        )
    };
    /// What one statement is to a counter c: c = const a, c = c ± const d, or c compared with const k, as
    /// `c < m` or `c >= m` (for integers `c <= k` is `c < k + 1`), or as `c == k` or `c != k` (no m of its own).
    enum Use {
        Set(Local, i128),
        Step(Local),
        Compare(Local, BinOp, i128),
    }
    // c op k, or k op c: as c < m or c >= m (Eq and Ne as they are, m = k)
    let normal = |op: &BinOp, k: i128, c_left: bool| -> (BinOp, i128) {
        let op = if c_left {
            *op
        } else {
            match op {
                BinOp::Lt => BinOp::Gt,
                BinOp::Le => BinOp::Ge,
                BinOp::Gt => BinOp::Lt,
                BinOp::Ge => BinOp::Le,
                op => *op,
            }
        };
        match op {
            BinOp::Le => (BinOp::Lt, k + 1),
            BinOp::Gt => (BinOp::Ge, k + 1),
            op => (op, k),
        }
    };
    let use_of = |s: &Statement| -> Option<Use> {
        let Statement::Assign(d, rv) = s else { return None };
        let d = d.as_local()?;
        match rv {
            Rvalue::Use(o) => int(o).map(|a| Use::Set(d, a)),
            Rvalue::BinaryOp(BinOp::Add | BinOp::Sub, x, y) if local(x) == Some(d) && int(y).is_some() => {
                Some(Use::Step(d))
            }
            Rvalue::BinaryOp(BinOp::Add, x, y) if local(y) == Some(d) && int(x).is_some() => Some(Use::Step(d)),
            Rvalue::BinaryOp(op, x, y) if comparison(op) => match (local(x), int(y), int(x), local(y)) {
                (Some(c), Some(k), _, _) if c != d => {
                    let (op, m) = normal(op, k, true);
                    Some(Use::Compare(c, op, m))
                }
                (_, _, Some(k), Some(c)) if c != d => {
                    let (op, m) = normal(op, k, false);
                    Some(Use::Compare(c, op, m))
                }
                _ => None,
            },
            _ => None,
        }
    };
    // each local: how often it is named, how often by a counter's statements, its constants, and its bound
    #[derive(Default)]
    struct Count {
        named: usize,
        counted: usize,
        sets: Vec<i128>,
        bound: Option<i128>,
        two_bounds: bool,
    }
    let mut counts: HashMap<Local, Count> = HashMap::new();
    for b in blocks.iter() {
        for s in &b.statements {
            s.visit_locals(&mut |l| counts.entry(l).or_default().named += 1);
            match use_of(s) {
                Some(Use::Set(c, a)) => {
                    let n = counts.entry(c).or_default();
                    n.counted += 1;
                    n.sets.push(a);
                }
                Some(Use::Step(c)) => counts.entry(c).or_default().counted += 2,
                Some(Use::Compare(c, op, m)) => {
                    let n = counts.entry(c).or_default();
                    n.counted += 1;
                    // (an equality is rebased with the bound the others give)
                    if !matches!(op, BinOp::Eq | BinOp::Ne) {
                        n.two_bounds |= n.bound.is_some_and(|b| b != m);
                        n.bound = Some(m);
                    }
                }
                None => {}
            }
        }
        b.terminator
            .visit_locals(&mut |l| counts.entry(l).or_default().named += 1);
    }
    let rebased: HashMap<Local, i128> = counts
        .into_iter()
        .filter_map(|(c, n)| {
            let k = n.bound?;
            let fits = n.sets.iter().all(|a| (-999..=999).contains(&(a - k)));
            let counter = matches!(types.get(&c), Some(ir::Ty::Int(_)));
            (k != 0
                && !n.two_bounds
                && !n.sets.is_empty()
                && n.named == n.counted
                && fits
                && counter
                && !keep.contains(&c))
            .then_some((c, k))
        })
        .collect();
    count_ends_down(blocks, types, keep, params, &rebased);
    if rebased.is_empty() {
        return vec![];
    }
    let card = |value: i128| Operand::Constant(Const::Int { value, ty: IntTy::I16 });
    for b in blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            let u = use_of(s);
            let Statement::Assign(_, rv) = s else { continue };
            match u {
                Some(Use::Set(c, a)) if rebased.contains_key(&c) => *rv = Rvalue::Use(card(a - rebased[&c])),
                Some(Use::Step(c)) if rebased.contains_key(&c) => {
                    if let Rvalue::BinaryOp(_, x, y) = rv {
                        for o in [x, y] {
                            if let Some(d) = int(o) {
                                *o = card(d);
                            }
                        }
                    }
                }
                Some(Use::Compare(c, op, m)) if rebased.contains_key(&c) => {
                    let at = m - rebased[&c];
                    *rv = Rvalue::BinaryOp(op, Operand::Copy(ir::Place::local(c)), card(at));
                }
                _ => {}
            }
        }
    }
    rebased.into_keys().collect()
}

/// `for _ in a..n` with n known only at run time: a counter c set to a and stepped by 1, whose only test is `c < e`,
/// e the range's end, read by nothing else. The loop counts e down instead (`e -= 1`, tested `a < e`), as
/// `(a..n).rev()` does, and c goes: no cell and no step of its own. That needs e to hold the bound whenever c is set,
/// and to stay put while the loop runs: from the block that sets c, no way back to it but through a line that sets
/// e, and from such a line no way to c's test but through c's block. e may be a parameter (set by each call) or set
/// on one arm of an `if` (`let n = if n >= 5 { n - 5 } else { n }`).
fn count_ends_down(
    blocks: &mut [Block],
    types: &HashMap<Local, ir::Ty>,
    keep: &HashSet<Local>,
    params: &HashSet<Local>,
    rebased: &HashMap<Local, i128>,
) {
    use ir::{BinOp, Const, Operand};
    let int = |o: &Operand| match o {
        Operand::Constant(Const::Int { value, .. }) => Some(*value),
        _ => None,
    };
    let local = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    let mut named: HashMap<Local, usize> = HashMap::new();
    // c: its sets (block, value); its steps; its tests, with their ends
    let mut sets: HashMap<Local, Vec<(usize, i128)>> = HashMap::new();
    let mut steps: HashMap<Local, usize> = HashMap::new();
    let mut tests: HashMap<Local, Vec<Local>> = HashMap::new();
    let mut defs: HashMap<Local, Vec<usize>> = HashMap::new();
    for (bi, b) in blocks.iter().enumerate() {
        for s in &b.statements {
            s.visit_locals(&mut |l| *named.entry(l).or_default() += 1);
            let Statement::Assign(d, rv) = s else { continue };
            let Some(d) = d.as_local() else { continue };
            defs.entry(d).or_default().push(bi);
            match rv {
                Rvalue::Use(o) if int(o).is_some() => sets.entry(d).or_default().push((bi, int(o).unwrap())),
                Rvalue::BinaryOp(BinOp::Add, x, y) if local(x) == Some(d) && int(y) == Some(1) => {
                    *steps.entry(d).or_default() += 1
                }
                Rvalue::BinaryOp(BinOp::Lt, x, y) => {
                    if let (Some(c), Some(e)) = (local(x), local(y)) {
                        if c != e && c != d && e != d {
                            tests.entry(c).or_default().push(e);
                        }
                    }
                }
                _ => {}
            }
        }
        b.terminator.visit_locals(&mut |l| *named.entry(l).or_default() += 1);
    }
    // e set by a call (`let n = inp();`): the call's block, and the block it returns to
    let mut called: HashMap<Local, Vec<(BasicBlock, BasicBlock)>> = HashMap::new();
    let mut preds: HashMap<BasicBlock, Vec<BasicBlock>> = HashMap::new();
    for b in blocks.iter() {
        if let Terminator::Call {
            dest, target: Some(t), ..
        } = &b.terminator
        {
            if let Some(d) = dest.as_local() {
                called.entry(d).or_default().push((b.name, *t));
            }
        }
        for s in b.terminator.successors() {
            preds.entry(s).or_default().push(b.name);
        }
    }
    let int_ty = |l: &Local| matches!(types.get(l), Some(ir::Ty::Int(_)));
    let mut chosen: Vec<(Local, Local, i128)> = vec![];
    for (c, ts) in &tests {
        let e = ts[0];
        let (Some([(cb, a)]), Some(st)) = (sets.get(c).map(|v| v.as_slice()), steps.get(c)) else {
            continue;
        };
        let c_only = named.get(c) == Some(&(1 + 2 * st + ts.len())) && ts.iter().all(|x| *x == e);
        // e: from c's block on, read by c's tests alone (and its own definitions), and fixed while c counts; what
        // reads it before then reads the bound as it was
        let e_once = only_tested(blocks, *cb, *c, e) && settled(blocks, *cb, *c, e, &defs, &called);
        let free = |l: &Local| !keep.contains(l) && !rebased.contains_key(l) && int_ty(l);
        // (a parameter may be counted down: each call sets it afresh, and nothing reads it after)
        let free_end = |l: &Local| (free(l) || params.contains(l)) && *l != Local(0);
        let unused = !chosen
            .iter()
            .any(|(x, y, _)| [x, y].contains(&c) || [x, y].contains(&&e));
        if c_only && e_once && free(c) && free_end(&e) && unused {
            chosen.push((*c, e, *a));
        }
    }
    for (c, e, a) in chosen {
        let k = |value: i128| {
            Operand::Constant(Const::Int {
                value,
                ty: ir::IntTy::I16,
            })
        };
        let step = Statement::Assign(
            ir::Place::local(e),
            Rvalue::BinaryOp(BinOp::Sub, Operand::Copy(ir::Place::local(e)), k(1)),
        );
        // the step first in the body, where the test's subtraction is the new e (as in `.rev()`), when the body's
        // first block is reached from the test alone: nothing in the body reads e
        let mut first = vec![];
        for b in blocks.iter() {
            if let (
                Some(Statement::Assign(t, Rvalue::BinaryOp(BinOp::Lt, x, y))),
                Terminator::SwitchInt {
                    discr,
                    arms,
                    otherwise: Some(body),
                },
            ) = (b.statements.last(), &b.terminator)
            {
                let tests_c = local(x) == Some(c) && local(y) == Some(e);
                let on_t = local(discr) == t.as_local() && arms.len() == 1 && arms[0].0 == 0;
                if tests_c && on_t && preds.get(body).map(|v| v.as_slice()) == Some(&[b.name][..]) {
                    first.push(*body);
                }
            }
        }
        let hoist = !first.is_empty() && first.len() == tests.get(&c).map_or(0, |t| t.len());
        for b in blocks.iter_mut() {
            b.statements
                .retain(|s| !matches!(s, Statement::Assign(d, Rvalue::Use(_)) if d.as_local() == Some(c)));
            for s in b.statements.iter_mut() {
                let Statement::Assign(d, rv) = s else { continue };
                if d.as_local() == Some(c) {
                    // c += 1: e -= 1
                    *s = if hoist { Statement::Nop } else { step.clone() };
                } else if let Rvalue::BinaryOp(BinOp::Lt, x, _) = rv {
                    if local(x) == Some(c) {
                        // c < e: a < e
                        *rv = Rvalue::BinaryOp(BinOp::Lt, k(a), Operand::Copy(ir::Place::local(e)));
                    }
                }
            }
            if hoist && first.contains(&b.name) {
                b.statements.insert(0, step.clone());
            }
            b.statements.retain(|s| !matches!(s, Statement::Nop));
        }
    }
}

/// Does e hold still while c counts, for count_ends_down? From c's block cb, no way back to cb but through a block that
/// sets e; and from a block that sets e, no way to a test of c but through cb. (A block that sets e and tests c is
/// taken to do both in the worse order.)
fn settled(
    blocks: &[Block],
    cb: usize,
    c: Local,
    e: Local,
    defs: &HashMap<Local, Vec<usize>>,
    called: &HashMap<Local, Vec<(BasicBlock, BasicBlock)>>,
) -> bool {
    use ir::BinOp;
    let index: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let mut sets: HashSet<usize> = defs.get(&e).map(|v| v.iter().copied().collect()).unwrap_or_default();
    sets.extend(
        called
            .get(&e)
            .into_iter()
            .flatten()
            .filter_map(|(p, _)| index.get(p).copied()),
    );
    let tests_c = |b: &Block| {
        b.statements.iter().any(|s| {
            matches!(s, Statement::Assign(_, Rvalue::BinaryOp(BinOp::Lt, x, _)) if x.place().and_then(|p| p.as_local()) == Some(c))
        })
    };
    // c's own block: nothing in it sets e after c is set (and if it sets e before, each way back to it sets e anew)
    let mut resets = false;
    if sets.contains(&cb) {
        let at_c = blocks[cb]
            .statements
            .iter()
            .position(|s| matches!(s, Statement::Assign(d, _) if d.as_local() == Some(c)));
        let at_e = blocks[cb]
            .statements
            .iter()
            .rposition(|s| matches!(s, Statement::Assign(d, _) if d.as_local() == Some(e)));
        if at_c.zip(at_e).is_some_and(|(ci, ei)| ei > ci)
            || called
                .get(&e)
                .is_some_and(|v| v.iter().any(|(p, _)| index.get(p) == Some(&cb)))
        {
            return false;
        }
        resets = at_e.is_some();
    }
    // the blocks reached from `from` without going through `stop` (nor past a block in `stop`)
    let reach = |from: usize, stop: &dyn Fn(usize) -> bool| -> HashSet<usize> {
        let mut seen = HashSet::new();
        let mut todo: Vec<usize> = blocks[from]
            .terminator
            .successors()
            .iter()
            .filter_map(|s| index.get(s).copied())
            .collect();
        while let Some(i) = todo.pop() {
            if !seen.insert(i) || stop(i) {
                continue;
            }
            todo.extend(
                blocks[i]
                    .terminator
                    .successors()
                    .iter()
                    .filter_map(|s| index.get(s).copied()),
            );
        }
        seen
    };
    // from cb, back to cb only through a block that sets e
    let from_c = reach(cb, &|i| sets.contains(&i) || i == cb);
    if !resets && from_c.contains(&cb) {
        return false;
    }
    // from a block that sets e, to c's tests only through cb (cb itself sets e before c, as checked above)
    for &d in sets.iter().filter(|&&d| d != cb) {
        if tests_c(&blocks[d]) {
            return false;
        }
        let after = reach(d, &|i| i == cb);
        if after.iter().any(|&i| i != cb && tests_c(&blocks[i])) {
            return false;
        }
    }
    true
}

/// From c's block cb on (cb, and every block reached from it), is e named only by c's tests `c < e` and by e's own
/// definitions? (Before cb, what reads e reads the bound as it was.)
fn only_tested(blocks: &[Block], cb: usize, c: Local, e: Local) -> bool {
    use ir::BinOp;
    let index: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let mut seen: HashSet<usize> = HashSet::from([cb]);
    let mut todo = vec![cb];
    while let Some(i) = todo.pop() {
        for s in blocks[i].terminator.successors() {
            if let Some(&j) = index.get(&s) {
                if seen.insert(j) {
                    todo.push(j);
                }
            }
        }
    }
    seen.iter().all(|&i| {
        let b = &blocks[i];
        let fine = |s: &Statement| match s {
            Statement::Assign(d, Rvalue::BinaryOp(BinOp::Lt, x, y)) => {
                (x.place().and_then(|p| p.as_local()) == Some(c)
                    && y.place().and_then(|p| p.as_local()) == Some(e)
                    && d.as_local() != Some(e))
                    || !s.mentions(e)
            }
            Statement::Assign(d, _) if d.as_local() == Some(e) => true,
            s => !s.mentions(e),
        };
        let call_sets = matches!(&b.terminator, Terminator::Call { dest, args, .. }
            if dest.as_local() == Some(e) && !args.iter().any(|a| a.mentions(e)));
        b.statements.iter().all(fine) && (call_sets || !b.terminator.mentions(e))
    })
}

/// Dead stores: `x = v` where no way on from it reads x before it is set again, as `if r.contains(&c)` leaves on its
/// false way (`_2 = const false`, then past the test that read it). Only a plain local whose address is never
/// taken, and only a value or a comparison, which do nothing else. Each return reads the result.
pub fn dead_stores(blocks: &mut [Block], results: &HashSet<Local>) {
    use ir::BinOp;
    let mut taken: HashSet<Local> = HashSet::new();
    for s in blocks.iter().flat_map(|b| &b.statements) {
        if let Statement::Assign(_, Rvalue::Ref(_, p)) = s {
            taken.extend(p.base_local());
        }
    }
    // a statement's reads, and the local it sets whole (if it does)
    let def_of = |s: &Statement| match s {
        Statement::Assign(d, _) => d.as_local(),
        _ => None,
    };
    let reads_of = |s: &Statement| -> Vec<Local> {
        let mut v = vec![];
        s.visit_locals(&mut |l| v.push(l));
        if let Some(d) = def_of(s) {
            // the destination itself is no read (it is named once, as the place written)
            if let Some(i) = v.iter().position(|x| *x == d) {
                v.remove(i);
            }
        }
        v
    };
    let term_reads = |t: &Terminator| -> (Vec<Local>, Option<Local>) {
        let mut v = vec![];
        t.visit_locals(&mut |l| v.push(l));
        let mut def = None;
        if let Terminator::Call { dest, .. } = t {
            if let Some(d) = dest.as_local() {
                if let Some(i) = v.iter().position(|x| *x == d) {
                    v.remove(i);
                }
                def = Some(d);
            }
        }
        if matches!(t, Terminator::Return) {
            v.push(Local(0));
            v.extend(results.iter().copied());
        }
        (v, def)
    };
    let index: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    // live on entry to each block, to a fixed point
    let mut live_in: Vec<HashSet<Local>> = vec![HashSet::new(); blocks.len()];
    loop {
        let mut changed = false;
        for i in (0..blocks.len()).rev() {
            let b = &blocks[i];
            let mut live: HashSet<Local> = HashSet::new();
            for s in b.terminator.successors() {
                if let Some(&j) = index.get(&s) {
                    live.extend(live_in[j].iter().copied());
                }
            }
            let (reads, def) = term_reads(&b.terminator);
            if let Some(d) = def {
                live.remove(&d);
            }
            live.extend(reads);
            for s in b.statements.iter().rev() {
                if let Some(d) = def_of(s) {
                    live.remove(&d);
                }
                live.extend(reads_of(s));
            }
            if live != live_in[i] {
                live_in[i] = live;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let pure = |rv: &Rvalue| match rv {
        Rvalue::Use(_) => true,
        Rvalue::BinaryOp(op, ..) => {
            matches!(
                op,
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
            )
        }
        _ => false,
    };
    for b in blocks.iter_mut() {
        let mut live: HashSet<Local> = HashSet::new();
        for s in b.terminator.successors() {
            if let Some(&j) = index.get(&s) {
                live.extend(live_in[j].iter().copied());
            }
        }
        let (reads, def) = term_reads(&b.terminator);
        if let Some(d) = def {
            live.remove(&d);
        }
        live.extend(reads);
        let mut keep = vec![true; b.statements.len()];
        for (k, s) in b.statements.iter().enumerate().rev() {
            if let (Some(d), Statement::Assign(_, rv)) = (def_of(s), s) {
                if !live.contains(&d) && !taken.contains(&d) && !results.contains(&d) && d != Local(0) && pure(rv) {
                    keep[k] = false;
                    continue;
                }
                live.remove(&d);
            }
            live.extend(reads_of(s));
        }
        let mut k = 0;
        b.statements.retain(|_| {
            k += 1;
            keep[k - 1]
        });
    }
}
