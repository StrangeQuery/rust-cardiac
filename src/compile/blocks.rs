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
pub fn dead_code(blocks: &mut [ir::Block], results: &HashSet<ir::Local>) {
    loop {
        // the lines that name each local, other than a statement assigning the whole of it
        let mut reading: HashMap<ir::Local, usize> = HashMap::new();
        for b in blocks.iter() {
            for line in b.each_line() {
                let assigned = match line {
                    Line::Statement(ir::Statement::Assign(p, _)) => p.as_local(),
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
                let Some(l) = p.as_local() else { return true };
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
