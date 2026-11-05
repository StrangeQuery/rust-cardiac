//! Joined tails. Once rustc has inlined a test it can answer again further on (`if v < 0` in `abs(v)`, then again
//! after it), it copies the code in between into both ways out of the first test (jump threading): two ways that
//! differ only in how they start and how they end. On CARDIAC that code can be a call to the multiply or divide
//! routine, each copy with its own argument stores, return and checks. Here the copies become one again: each way
//! keeps its own start and goes on to one copy of the code they share, which ends by making the test again (what
//! it reads has not changed since) and going on to each way's own end.
use std::collections::{HashMap, HashSet};

use super::ir::{predecessors, BasicBlock, BinOp, Block, Local, Operand, Place, Rvalue, Statement, Terminator, Ty};
use super::util::Ordered;

/// One step of a way: a statement, or a terminator that goes on to the way's next block (an assert, a call), its
/// target left unset so that two ways' steps compare equal.
#[derive(Clone, PartialEq)]
enum Step {
    Statement(Statement),
    On(Terminator),
}

/// Where a step's terminator goes on to, until it is given its block.
const UNSET: BasicBlock = BasicBlock(u32::MAX);

/// The shared code must be worth at least this many cells: the second test (CLA, TAC), the jump from the first
/// way's start, and what keeping the test's operand and the starts' results apart costs. A call to the routine
/// alone is not enough (measured: two or three cells more); a call and anything else is.
const LEAST: usize = 8;

impl Step {
    /// Does it write `l`, the whole local or a part of it (or end its life)? A line the IR does not model may.
    fn writes(&self, l: Local) -> bool {
        match self {
            Step::Statement(Statement::Assign(p, _)) => p.base_local() == Some(l),
            Step::Statement(Statement::StorageLive(x) | Statement::StorageDead(x)) => *x == l,
            Step::Statement(Statement::Unsupported(_)) => true,
            Step::Statement(Statement::Assume(_) | Statement::Nop) => false,
            Step::On(Terminator::Call { dest, .. }) => dest.base_local() == Some(l),
            Step::On(_) => false,
        }
    }

    /// About how many cells it takes: a multiply or divide by a local, the call to the routine with the arguments
    /// passed and the result kept; a check, its test and the jump to the panic; a call, its arguments and the jump;
    /// a copy, a load and a store; anything else, a load, the work and a store.
    fn worth(&self) -> usize {
        let local = |o: &Operand| !matches!(o, Operand::Constant(_));
        match self {
            Step::Statement(Statement::Assign(_, Rvalue::BinaryOp(op, x, y))) => match op {
                BinOp::Mul | BinOp::MulWithOverflow if local(x) && local(y) => 6,
                BinOp::Div | BinOp::Rem | BinOp::UDiv | BinOp::URem if local(y) => 6,
                _ => 3,
            },
            Step::Statement(Statement::Assign(_, Rvalue::Use(_))) => 2,
            Step::Statement(_) => 3,
            Step::On(Terminator::Assert { .. }) => 4,
            Step::On(Terminator::Call { args, .. }) => 2 + 2 * args.len(),
            Step::On(_) => 3,
        }
    }

    /// A comparison into a bool, which the lowering keeps beside the test of it that follows.
    fn compares(&self) -> bool {
        matches!(self, Step::Statement(Statement::Assign(_, Rvalue::BinaryOp(op, ..))) if comparison(*op))
    }
}

fn comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
    )
}

/// A way out of a branch: its blocks, each reached only from the one before (the first only from the branch), and
/// their steps, up to the terminator that leaves them.
struct Way {
    blocks: Vec<BasicBlock>,
    steps: Vec<Step>,
    exit: Terminator,
}

/// The way that starts at `first`, out of the branch at `from`, as far as it goes block by block: through a goto,
/// an assert or a call (not one that ends a phase or loads a resource) to a block that nothing else leads to.
fn way(blocks: &[Block], preds: &HashMap<BasicBlock, Vec<BasicBlock>>, first: BasicBlock, from: BasicBlock) -> Way {
    let entry = blocks[0].name;
    let mut w = Way {
        blocks: vec![],
        steps: vec![],
        exit: Terminator::Unreachable,
    };
    let mut cur = first;
    while let Some(b) = blocks.iter().find(|b| b.name == cur) {
        w.blocks.push(cur);
        w.steps.extend(b.statements.iter().cloned().map(Step::Statement));
        let next = match &b.terminator {
            Terminator::Goto(t) | Terminator::Assert { target: t, .. } => Some(*t),
            Terminator::Call {
                func, target: Some(t), ..
            } if !["overlay", "load_resource"].contains(&func.name()) => Some(*t),
            _ => None,
        }
        .filter(|t| {
            *t != from
                && *t != entry
                && !w.blocks.contains(t)
                && b.terminator.successors() == [*t]
                && preds.get(t).is_some_and(|p| p.as_slice() == [cur])
        });
        let Some(t) = next else {
            w.exit = b.terminator.clone();
            break;
        };
        let mut on = b.terminator.clone();
        match &mut on {
            Terminator::Assert { target, .. } => *target = UNSET,
            Terminator::Call { target, .. } => *target = Some(UNSET),
            _ => {}
        }
        if !matches!(on, Terminator::Goto(_)) {
            w.steps.push(Step::On(on));
        }
        cur = t;
    }
    w
}

/// Blocks that run `steps` and then `end`: the first named `name`, the others new, each step's terminator going on
/// to the next.
fn build(name: BasicBlock, steps: &[Step], end: Terminator, fresh: &mut u32) -> Vec<Block> {
    let new = |name| Block {
        name,
        statements: vec![],
        terminator: Terminator::Unreachable,
    };
    let mut out = vec![];
    let mut cur = new(name);
    for s in steps {
        match s {
            Step::Statement(s) => cur.statements.push(s.clone()),
            Step::On(t) => {
                let next = BasicBlock(*fresh);
                *fresh += 1;
                let mut t = t.clone();
                match &mut t {
                    Terminator::Assert { target, .. } => *target = next,
                    Terminator::Call { target, .. } => *target = Some(next),
                    _ => unreachable!(),
                }
                cur.terminator = t;
                out.push(std::mem::replace(&mut cur, new(next)));
            }
        }
    }
    cur.terminator = end;
    out.push(cur);
    out
}

/// How a branch's test is made again after the shared code: the comparison it was worked out from, or the bool.
enum Retest {
    Compare(BinOp, Operand, Operand),
    Again(Local),
}

impl Retest {
    /// The locals it reads, which nothing on the way from the test to the second one may write.
    fn reads(&self) -> Vec<Local> {
        match self {
            Retest::Compare(_, x, y) => [x, y].iter().filter_map(|o| o.local()).collect(),
            Retest::Again(c) => vec![*c],
        }
    }
}

/// The test a block's two-way branch makes, `switchInt(c) -> [0: no, otherwise: yes]`, as it can be made again
/// later: the comparison `c` was worked out from in this block, of copies of whole locals and constants, when
/// nothing in the block writes them after; else `c` itself. With the two ways.
fn branch(b: &Block) -> Option<(Retest, BasicBlock, BasicBlock)> {
    let Terminator::SwitchInt {
        discr: Operand::Copy(p) | Operand::Move(p),
        arms,
        otherwise: Some(yes),
    } = &b.terminator
    else {
        return None;
    };
    let (Some(c), [(0, no)]) = (p.as_local(), arms.as_slice()) else {
        return None;
    };
    let side = |o: &Operand| match o {
        Operand::Constant(k) if k.is_scalar() => Some(o.clone()),
        Operand::Copy(_) => o
            .local()
            .filter(|l| *l != Local::UNIT)
            .map(|l| Operand::Copy(Place::local(l))),
        _ => None,
    };
    let at = b.statements.iter().rposition(|s| s.def() == Some(c));
    let compared = at.and_then(|k| match &b.statements[k] {
        Statement::Assign(_, Rvalue::BinaryOp(op, x, y)) if comparison(*op) => {
            let t = Retest::Compare(*op, side(x)?, side(y)?);
            let after: Vec<Step> = b.statements[k + 1..].iter().cloned().map(Step::Statement).collect();
            (!t.reads().iter().any(|l| after.iter().any(|s| s.writes(*l)))).then_some(t)
        }
        _ => None,
    });
    Some((compared.unwrap_or(Retest::Again(c)), *yes, *no))
}

/// The run of steps two ways share that is worth most, as (where it starts in each, how long, its worth): as far
/// as nothing up to its end writes what `keep` names (what the second test reads), and not ending in a comparison
/// (kept beside its test, which differs).
fn shared(a: &[Step], b: &[Step], keep: &[Local]) -> Option<(usize, usize, usize, usize)> {
    let clean = |ss: &[Step]| ss.iter().all(|s| !keep.iter().any(|l| s.writes(*l)));
    let mut best: Option<(usize, usize, usize, usize)> = None;
    for i in 0..a.len() {
        for j in 0..b.len() {
            let n = a[i..].iter().zip(&b[j..]).take_while(|(x, y)| x == y).count();
            let n = (1..=n)
                .rev()
                .find(|&n| clean(&a[..i + n]) && clean(&b[..j + n]) && !a[i + n - 1].compares());
            let Some(n) = n else { continue };
            let w: usize = a[i..i + n].iter().map(Step::worth).sum();
            if best.is_none_or(|(.., most)| w > most) {
                best = Some((i, j, n, w));
            }
        }
    }
    best
}

/// Joined tails: for each two-way branch, the steps both its ways run (the same statements, checks and calls, in
/// the same order, on the same locals) are run once, in blocks of their own that both ways go on to, each way
/// keeping what it ran before them; the shared blocks end by making the branch's test again and going on to what
/// each way ran after them. Each way still does the same work in the same order, from the same values, so it is
/// sound when the second test gives the first one's answer: what it reads (whole locals whose address is never
/// taken) nothing writes in between. It pays when the shared steps are worth more than the second test and the
/// jumps around it (`LEAST`). Runs after the range pass, which has dropped the checks each way's own facts prove
/// (the zero check after a promise, `MIN / -1`): only code both ways still run is joined, so a check dropped on
/// one way and kept on the other stays where it is, and none comes back.
pub fn join_tails(blocks: &mut Vec<Block>, locals: &mut Ordered<Local, Ty>) {
    let taken: HashSet<Local> = blocks
        .iter()
        .flat_map(|b| &b.statements)
        .filter_map(|s| match s {
            Statement::Assign(_, Rvalue::Ref(_, p)) => p.base_local(),
            _ => None,
        })
        .collect();
    let mut local = 1 + locals
        .keys()
        .filter(|l| **l != Local::UNIT)
        .map(|l| l.0)
        .max()
        .unwrap_or(0);
    let mut fresh = 1 + blocks.iter().map(|b| b.name.0).max().unwrap_or(0).max(900000);
    loop {
        let preds = predecessors(blocks);
        let found = blocks.iter().find_map(|h| {
            let (test, yes, no) = branch(h)?;
            let only = |n: BasicBlock| preds.get(&n).is_some_and(|p| p.as_slice() == [h.name]);
            let reads = test.reads();
            if yes == no || yes == h.name || no == h.name || !only(yes) || !only(no) {
                return None;
            }
            if reads.iter().any(|l| taken.contains(l)) {
                return None;
            }
            let (a, b) = (way(blocks, &preds, yes, h.name), way(blocks, &preds, no, h.name));
            let (i, j, n, w) = shared(&a.steps, &b.steps, &reads)?;
            (w >= LEAST).then_some((test, a, b, i, j, n))
        });
        let Some((test, a, b, i, j, n)) = found else {
            break;
        };
        let (join, yes_end, no_end) = (BasicBlock(fresh), BasicBlock(fresh + 1), BasicBlock(fresh + 2));
        fresh += 3;
        let mut middle = a.steps[i..i + n].to_vec();
        let discr = match test {
            Retest::Again(c) => Operand::Copy(Place::local(c)),
            Retest::Compare(op, x, y) => {
                let t = Local(local);
                local += 1;
                locals.set(t, Ty::Bool);
                middle.push(Step::Statement(Statement::Assign(
                    Place::local(t),
                    Rvalue::BinaryOp(op, x, y),
                )));
                Operand::Move(Place::local(t))
            }
        };
        let retest = Terminator::SwitchInt {
            discr,
            arms: vec![(0, no_end)],
            otherwise: Some(yes_end),
        };
        // the second way's start falls into the shared code, and that into the second way's end (the test's 0 arm)
        let mut new = build(a.blocks[0], &a.steps[..i], Terminator::Goto(join), &mut fresh);
        new.extend(build(b.blocks[0], &b.steps[..j], Terminator::Goto(join), &mut fresh));
        new.extend(build(join, &middle, retest, &mut fresh));
        new.extend(build(no_end, &b.steps[j + n..], b.exit, &mut fresh));
        new.extend(build(yes_end, &a.steps[i + n..], a.exit, &mut fresh));
        let gone = |x: &Block| a.blocks.contains(&x.name) || b.blocks.contains(&x.name);
        let at = blocks.iter().position(gone).unwrap();
        blocks.retain(|x| !gone(x));
        blocks.splice(at..at, new);
    }
}
