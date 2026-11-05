//! Passes on arithmetic: sum chains, linear folding, repeated work, and comparisons made again where they are
//! tested.
use std::collections::{BTreeMap, HashMap, HashSet};

use super::ir::{
    find, lines_naming, occurrences, predecessors, BinOp, Block, Const, IntTy, Local, Locals, Operand, Place,
    PlaceBase, Projection, Rvalue, Sign, Statement, Terminator, Ty, UnOp, Unwind,
};
use super::util::Ordered;

/// `assert(!move (_t.1: bool), ..) -> [success: bb]`: the check of an overflow-checked op's flag, read by move.
fn moved_overflow_assert(t: &Terminator) -> Option<(Local, super::ir::BasicBlock)> {
    let Terminator::Assert {
        cond: Operand::Move(p),
        expected: false,
        target,
        unwind: Unwind::Continue | Unwind::Unreachable,
        ..
    } = t
    else {
        return None;
    };
    match p.projection.as_slice() {
        [Projection::Field(1, Ty::Bool)] => p.base_local().map(|l| (l, *target)),
        _ => None,
    }
}

fn i16_pair(types: &HashMap<Local, Ty>, x: Local) -> bool {
    types.get(&x) == Some(&Ty::Tuple(vec![Ty::Int(IntTy::I16), Ty::Bool]))
}

/// Sum chains. a + b + c - d in Rust is one overflow-checked op per term, each result feeding the next. On CARDIAC
/// the accumulator holds four digits, so a chain of up to nine card values can be added up in it with nothing
/// stored between, and checked once when the total is stored: a total past +-999 panics, as any i16 a card cannot
/// hold does, and partial sums are exact, as they are in native Rust.
pub fn sum_chains(blocks: &mut Vec<Block>, types: &HashMap<Local, Ty>) {
    let mut changed = true;
    while changed {
        changed = false;
        for bi in 0..blocks.len() {
            let Some(Statement::Assign(dest, rv)) = blocks[bi].statements.last() else {
                continue;
            };
            let Some(m) = dest.as_local().filter(|l| *l != Local::UNIT) else {
                continue;
            };
            let terms: Vec<(Sign, Operand)> = match rv {
                Rvalue::Sum(terms) => terms.clone(),
                Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow), a, b) => {
                    vec![
                        (Sign::Plus, a.clone()),
                        (
                            if *op == BinOp::AddWithOverflow {
                                Sign::Plus
                            } else {
                                Sign::Minus
                            },
                            b.clone(),
                        ),
                    ]
                }
                _ => continue,
            };
            let Some((t, succ)) = moved_overflow_assert(&blocks[bi].terminator) else {
                continue;
            };
            if t != m || !i16_pair(types, m) {
                continue;
            }
            let Some(si) = find(blocks, succ) else { continue };
            if si == bi {
                continue;
            }
            if (0..blocks.len())
                .filter(|&x| x != si && blocks[x].terminator.names(succ))
                .count()
                != 1
            {
                continue;
            }
            // (an assert's message arguments are only for printing: they do not count)
            let reads: usize = blocks
                .iter()
                .map(|b| {
                    b.statements
                        .iter()
                        .filter(|s| s.places().iter().any(|p| p.is_value_of(m)))
                        .count()
                        + (!matches!(b.terminator, Terminator::Assert { .. })
                            && b.terminator.places().iter().any(|p| p.is_value_of(m)))
                            as usize
                })
                .sum();
            let field = Operand::Copy(Place::field(m, 0, Ty::Int(IntTy::I16)));
            let Some(Statement::Assign(
                udest,
                Rvalue::BinaryOp(uop @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow), ua, ub),
            )) = blocks[si].statements.first()
            else {
                continue;
            };
            let Some(u) = udest.as_local().filter(|l| *l != Local::UNIT) else {
                continue;
            };
            if reads != 1 || !i16_pair(types, u) {
                continue;
            }
            if terms.len() >= 9 {
                continue;
            }
            let usign = if *uop == BinOp::AddWithOverflow {
                Sign::Plus
            } else {
                Sign::Minus
            };
            let next: Vec<(Sign, Operand)> = if *ua == field {
                terms.into_iter().chain(std::iter::once((usign, ub.clone()))).collect()
            } else if *ub == field && *uop == BinOp::AddWithOverflow {
                std::iter::once((Sign::Plus, ua.clone())).chain(terms).collect()
            } else {
                continue;
            };
            let s = blocks.remove(si);
            let bi = if si < bi { bi - 1 } else { bi };
            let b = &mut blocks[bi];
            *b.statements.last_mut().unwrap() = Statement::Assign(Place::local(u), Rvalue::Sum(next));
            // the chain's last op is still checked by S's own assert, now B's terminator
            b.statements.extend(s.statements.into_iter().skip(1));
            b.terminator = s.terminator;
            changed = true;
            break;
        }
    }
}

/// One term of a linear sum: a coefficient and an operand.
type Term = (i64, Operand);

#[derive(Clone)]
struct Lin {
    terms: Vec<Term>,
    line: usize,
    plain: bool,
    folded: bool,
    /// a local its terms read was assigned since: its value is there in its own local, but its terms are no
    /// longer what it was worked out from, so it is read, not absorbed
    stale: bool,
}

/// Linear folding (release builds: plain Add, Sub, Neg and Mul by a small constant). An expression of single-use
/// temporaries such as 3 (a + b) + 2 (c - d) becomes one Lin(+3*a, +3*b, +2*c, -2*d), added up in the accumulator,
/// which holds four digits: so the total, and any partial sum, may pass what a card holds, as in native Rust, as
/// long as the coefficients add up to 10 or less (10 x 999 still fits). The total is stored, or carried straight
/// into the next statement (a / 10, say) by accumulator forwarding.
pub fn lin_fold(blocks: &mut [Block], types: &HashMap<Local, Ty>, named: &HashSet<Local>) {
    for bi in 0..blocks.len() {
        let mut lin: Ordered<Local, Lin> = Ordered::new();
        // a local assigned again in the block: its earlier sums, each still to be rewritten at its own line
        let mut earlier: Vec<(Local, Lin)> = vec![];
        let mut absorbed: Vec<Local> = vec![];
        for i in 0..blocks[bi].statements.len() {
            // what the line before assigned: a sum read from it before then can't be worked out again here
            if let Some(Statement::Assign(p, _)) = i.checked_sub(1).map(|k| &blocks[bi].statements[k]) {
                if let Some(v) = p.base_local() {
                    let keys: Vec<Local> = lin.keys().copied().collect();
                    for t in keys {
                        let e = lin.get_mut(&t).unwrap();
                        if e.terms.iter().any(|(_, o)| o.local() == Some(v)) {
                            e.stale = true;
                        }
                    }
                }
            }
            let candidate = |t: Local| t != Local(0) && !named.contains(&t) && occurrences(blocks, t) == 2;
            // an operand as terms: a foldable temporary's own terms, or the operand itself
            let terms_of = |lin: &Ordered<Local, Lin>, op: &Operand| -> Option<(Vec<Term>, Option<Local>)> {
                let x = match op {
                    Operand::Copy(p) | Operand::Move(p) => p.as_local().filter(|l| *l != Local::UNIT)?,
                    _ => return None,
                };
                // a plain temporary keeps its own line, so it is read, never absorbed
                if let Some(e) = lin.get(&x) {
                    if !e.plain && !e.stale && candidate(x) {
                        return Some((e.terms.clone(), Some(x)));
                    }
                }
                Some((vec![(1, Operand::Copy(Place::local(x)))], None))
            };
            let Statement::Assign(d, rv) = &blocks[bi].statements[i] else {
                continue;
            };
            let Some(dest) = d.as_local().filter(|l| *l != Local::UNIT) else {
                continue;
            };
            let scale = |p: (Vec<Term>, Option<Local>), k: i64| {
                (p.0.into_iter().map(|(c, v)| (c * k, v)).collect::<Vec<_>>(), p.1)
            };
            let parts = match rv {
                Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub), a, b) => {
                    match (terms_of(&lin, a), terms_of(&lin, b)) {
                        (Some(x), Some(y)) => Some(vec![x, scale(y, if *op == BinOp::Sub { -1 } else { 1 })]),
                        _ => None,
                    }
                }
                // (by 10 a decimal shift is cheaper than ten adds: left alone)
                Rvalue::BinaryOp(BinOp::Mul, a, Operand::Constant(Const::Int { value, ty: IntTy::I16 })) => {
                    let k = *value as i64;
                    terms_of(&lin, a).filter(|_| k.abs() < 10).map(|x| vec![scale(x, k)])
                }
                Rvalue::UnaryOp(UnOp::Neg, a) => terms_of(&lin, a).map(|x| vec![scale(x, -1)]),
                _ => continue,
            };
            let Some(parts) = parts else { continue };
            if types.get(&dest) != Some(&Ty::Int(IntTy::I16)) {
                continue;
            }
            if let Some(e) = lin.get(&dest) {
                earlier.push((dest, e.clone()));
            }
            let terms: Vec<Term> = parts.iter().flat_map(|p| p.0.iter().cloned()).collect();
            if terms.iter().map(|(k, _)| k.abs()).sum::<i64>() > 10 {
                // too much for the accumulator: keep the operands as they are, this statement as it is
                lin.set(
                    dest,
                    Lin {
                        terms: vec![(1, Operand::Copy(Place::local(dest)))],
                        line: i,
                        plain: true,
                        folded: false,
                        stale: false,
                    },
                );
                continue;
            }
            for p in &parts {
                if let Some(f) = p.1 {
                    if !absorbed.contains(&f) {
                        absorbed.push(f);
                    }
                }
            }
            let folded = parts.iter().any(|p| p.1.is_some());
            lin.set(
                dest,
                Lin {
                    terms,
                    line: i,
                    plain: false,
                    folded,
                    stale: false,
                },
            );
        }
        let drop: HashSet<usize> = absorbed.iter().map(|t| lin.get(t).unwrap().line).collect();
        for (t, x) in earlier.iter().map(|(t, x)| (t, x)).chain(lin.iter()) {
            if absorbed.contains(t) || x.plain || !x.folded {
                continue;
            }
            blocks[bi].statements[x.line] = Statement::Assign(Place::local(*t), Rvalue::Lin(x.terms.clone()));
        }
        let statements = std::mem::take(&mut blocks[bi].statements);
        blocks[bi].statements = statements
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !drop.contains(i))
            .map(|(_, s)| s)
            .collect();
    }
}

/// The locals whose address is taken as a whole or at an index: `&x`, `&mut x`, `&raw mut x[0 of 1]`.
fn addressed(blocks: &[Block]) -> HashSet<Local> {
    let mut taken = HashSet::new();
    for s in blocks.iter().flat_map(|b| &b.statements) {
        if let Statement::Assign(_, Rvalue::Ref(_, p)) = s {
            if let (Some(l), 0) = (p.base_local(), p.wrapped()) {
                taken.insert(l);
            }
        }
    }
    taken
}

/// What a value is worked out from, as repeated work compares it: a local, or an integer constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arg {
    Local(Local),
    Int(i128, IntTy),
}

/// An expression worked out into a local: `_x = Op(a, b)` or `_x = Neg(a)`.
#[derive(Clone, Debug, PartialEq)]
struct Expr {
    dest: Local,
    key: Key,
}

/// An operation and what it is worked out from.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    op: &'static str,
    args: Vec<Arg>,
}

impl Key {
    fn reads(&self, v: Local) -> bool {
        self.args.contains(&Arg::Local(v))
    }
}

/// The local the line's assignment starts from, as repeated work tracks what changes: a whole local, or one
/// projected without more than one field or downcast around it.
fn assigned_base(p: &Place) -> Option<Local> {
    let l = p.base_local().filter(|l| *l != Local::UNIT)?;
    let wraps: Vec<&Projection> = p
        .projection
        .iter()
        .filter(|x| matches!(x, Projection::Deref | Projection::Field(..) | Projection::Downcast(_)))
        .collect();
    match wraps.as_slice() {
        [] | [Projection::Field(..)] | [Projection::Downcast(_)] => Some(l),
        _ => None,
    }
}

/// Repeated work. A sum, difference, product, quotient, remainder or negation worked out again from the same values
/// (an inlined helper called twice with the same arguments, say) is taken from the local that already holds it,
/// when every way into that point has worked it out into the same local and nothing has changed since. Copy
/// forwarding then usually removes the copy. Comparisons are left alone: testing again is cheaper than keeping a
/// bool. Only plain locals take part, never one whose address is taken.
pub fn merge_repeats(blocks: &mut [Block]) {
    let addressed = addressed(blocks);
    let arg = |o: &Operand| -> Option<Arg> {
        if let Some((v, t)) = o.int() {
            return Some(Arg::Int(v, t));
        }
        o.local()
            .filter(|l| *l != Local::UNIT && !addressed.contains(l))
            .map(Arg::Local)
    };
    let expr = |s: &Statement| -> Option<Expr> {
        let Statement::Assign(d, rv) = s else { return None };
        let dest = d.as_local().filter(|l| *l != Local::UNIT)?;
        let (op, ops): (&'static str, Vec<&Operand>) = match rv {
            Rvalue::BinaryOp(
                op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::UDiv | BinOp::URem),
                a,
                b,
            ) => (op.name(), vec![a, b]),
            Rvalue::UnaryOp(UnOp::Neg, a) => ("Neg", vec![a]),
            _ => return None,
        };
        if addressed.contains(&dest) {
            return None;
        }
        let args: Option<Vec<Arg>> = ops.into_iter().map(arg).collect();
        Some(Expr {
            dest,
            key: Key { op, args: args? },
        })
    };
    let assigned = |line: Line| -> Option<Local> {
        match line {
            Line::Statement(Statement::Assign(p, _)) => assigned_base(p),
            Line::Terminator(Terminator::Call { dest, .. }) => assigned_base(dest),
            _ => None,
        }
    };
    let preds = predecessors(blocks);
    let kill = |av: &mut Held, keys: &[Key], v: Local| {
        av.retain(|k, h| *h != v && !keys[k.0].reads(v));
    };
    // Keys are interned so that the maps of what is held compare cheaply.
    let mut keys: Vec<Key> = vec![];
    for b in blocks.iter() {
        for s in &b.statements {
            if let Some(e) = expr(s) {
                if !keys.contains(&e.key) {
                    keys.push(e.key);
                }
            }
        }
    }
    let id = |k: &Key| KeyId(keys.iter().position(|x| x == k).unwrap());
    // what is held on the way out of each block (missing: not reached yet, so it doesn't narrow a join)
    let walk = |b: &mut Block, mut av: Held, rewrite: bool| -> Held {
        let n = b.statements.len();
        for i in 0..=n {
            let e = if i < n { expr(&b.statements[i]) } else { None };
            if let Some(e) = &e {
                let k = id(&e.key);
                if let Some(h) = av.get(&k).copied() {
                    if h != e.dest {
                        if rewrite {
                            b.statements[i] =
                                Statement::Assign(Place::local(e.dest), Rvalue::Use(Operand::Copy(Place::local(h))));
                        }
                        kill(&mut av, &keys, e.dest);
                        if !e.key.reads(h) {
                            av.insert(k, h);
                        }
                        continue;
                    }
                }
            }
            let line = if i < n {
                Line::Statement(&b.statements[i])
            } else {
                Line::Terminator(&b.terminator)
            };
            if let Some(v) = assigned(line) {
                kill(&mut av, &keys, v);
            }
            if let Some(e) = e {
                if !e.key.reads(e.dest) {
                    av.insert(id(&e.key), e.dest);
                }
            }
        }
        av
    };
    // on the way into a block: what every reached predecessor holds in the same local; nothing into the entry block
    let into = |out: &HashMap<super::ir::BasicBlock, Held>, bi: usize, b: &Block| -> Held {
        let ins: Vec<&Held> = preds
            .get(&b.name)
            .into_iter()
            .flatten()
            .filter_map(|p| out.get(p))
            .collect();
        if bi == 0 || ins.is_empty() {
            return Held::new();
        }
        let mut av = ins[0].clone();
        for o in &ins[1..] {
            av.retain(|k, h| o.get(k) == Some(h));
        }
        av
    };
    let mut out: HashMap<super::ir::BasicBlock, Held> = HashMap::new();
    let mut changed = true;
    let mut round = 0;
    while changed && round < 50 {
        changed = false;
        for (bi, b) in blocks.iter().enumerate() {
            let o = walk(&mut b.clone(), into(&out, bi, b), false);
            if out.get(&b.name) != Some(&o) {
                out.insert(b.name, o);
                changed = true;
            }
        }
        round += 1;
    }
    if changed {
        return; // not settled: leave the code as it is
    }
    // find every reuse first, then keep only those that make the program smaller
    struct Reuse {
        b: usize,
        i: usize,
        line: Statement,
        holder: Local,
        was: Statement,
    }
    let mut reuses = vec![];
    for (bi, b) in blocks.iter().enumerate() {
        let mut now = b.clone();
        walk(&mut now, into(&out, bi, b), true);
        for (i, s) in now.statements.into_iter().enumerate() {
            if s != b.statements[i] {
                let Statement::Assign(_, Rvalue::Use(Operand::Copy(h))) = &s else {
                    unreachable!()
                };
                let holder = h.as_local().unwrap();
                reuses.push(Reuse {
                    b: bi,
                    i,
                    line: s,
                    holder,
                    was: b.statements[i].clone(),
                });
            }
        }
    }
    // A reuse saves working the value out again: about a cell, more for a product or quotient of two locals (that
    // runs a routine). A holder only ever read as the first operand of the very next line never left the
    // accumulator; keeping it for later costs a store and a cell of its own. Any other holder is stored already.
    let saves = |s: &Statement| match s {
        Statement::Assign(
            _,
            Rvalue::BinaryOp(BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::UDiv | BinOp::URem, a, b),
        ) if a.local().is_some() && b.local().is_some() => 3,
        _ => 1,
    };
    let in_acc = |h: Local| {
        blocks.iter().all(|x| {
            let defs = x.defs();
            let lines: Vec<&dyn Locals> = x.lines().collect();
            (0..lines.len()).all(|i| {
                let line = if i < x.statements.len() {
                    Line::Statement(&x.statements[i])
                } else {
                    Line::Terminator(&x.terminator)
                };
                !lines[i].mentions(h)
                    || defs[i] == Some(h)
                    || (i > 0 && defs[i - 1] == Some(h) && first_operand_is(line, h) && lines[i].count(h) == 1)
            })
        })
    };
    let mut holders = HashSet::new();
    let mut order: Vec<Local> = vec![];
    for r in &reuses {
        if !order.contains(&r.holder) {
            order.push(r.holder);
        }
    }
    for h in order {
        let total: i32 = reuses.iter().filter(|r| r.holder == h).map(|r| saves(&r.was)).sum();
        if total > if in_acc(h) { 2 } else { 0 } {
            holders.insert(h);
        }
    }
    for r in &reuses {
        if holders.contains(&r.holder) {
            blocks[r.b].statements[r.i] = r.line.clone();
        }
    }
    // a holder read again later is no longer read for the last time where rustc moved it
    if !holders.is_empty() {
        let to_copy = |o: &mut Operand| {
            if let Operand::Move(p) = o {
                if p.wrapped() == 0 && p.base_local().is_some_and(|l| holders.contains(&l)) {
                    *o = Operand::Copy(p.clone());
                }
            }
        };
        for b in blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.operands_mut().into_iter().for_each(to_copy);
            }
            b.terminator.operands_mut().into_iter().for_each(to_copy);
        }
    }
    // a bool tested by one switch, and nothing else, is that switch's alone: `copy` there means the same as `move`,
    // and lets the comparison and the branch become one (rustc sometimes copies one block into two, so the same bool
    // is set and tested in each: every such pair counts)
    let switched = |b: &Block| match &b.terminator {
        Terminator::SwitchInt {
            discr: Operand::Copy(p) | Operand::Move(p),
            ..
        } => p.as_local(),
        _ => None,
    };
    for bi in 0..blocks.len() {
        let Terminator::SwitchInt {
            discr: Operand::Copy(p),
            ..
        } = &blocks[bi].terminator
        else {
            continue;
        };
        let Some(v) = p.as_local() else { continue };
        let pairs: Vec<usize> = (0..blocks.len())
            .filter(|&x| {
                blocks[x].statements.last().and_then(Statement::def) == Some(v) && switched(&blocks[x]) == Some(v)
            })
            .collect();
        if pairs.contains(&bi) && lines_naming(blocks, v) == 2 * pairs.len() {
            if let Terminator::SwitchInt { discr, .. } = &mut blocks[bi].terminator {
                *discr = Operand::Move(Place::local(v));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct KeyId(usize);

/// What repeated work knows is held at a point: each worked-out expression and the local holding it. Compared as a
/// map, not a list, so that the analysis settles once the facts stop changing, whatever order they came in.
type Held = BTreeMap<KeyId, Local>;

/// One line of a block, by reference.
#[derive(Clone, Copy)]
enum Line<'a> {
    Statement(&'a Statement),
    Terminator(&'a Terminator),
}

/// Is `h`, copied or moved whole, the first operand inside a line's parentheses: `Op(copy h, ..)`, `f(move h)`,
/// `switchInt(move h)`, `assert(copy h, ..)`?
fn first_operand_is(line: Line, h: Local) -> bool {
    let first: Option<&Operand> = match line {
        Line::Statement(Statement::Assign(_, rv)) => match rv {
            Rvalue::BinaryOp(_, a, _) | Rvalue::UnaryOp(_, a) => Some(a),
            Rvalue::Tuple(os) | Rvalue::Variant(_, os) => os.first(),
            _ => None,
        },
        Line::Statement(Statement::Assume(o)) => Some(o),
        Line::Statement(_) => None,
        Line::Terminator(Terminator::SwitchInt { discr, .. }) => Some(discr),
        Line::Terminator(Terminator::Call { args, .. }) => args.first(),
        Line::Terminator(Terminator::Assert {
            cond, expected: true, ..
        }) => Some(cond),
        Line::Terminator(_) => None,
    };
    matches!(first, Some(Operand::Copy(p) | Operand::Move(p)) if p.as_local() == Some(h))
}

/// Rematerialization: rustc works a comparison out once and tests the bool in several places (`if p < 0` twice).
/// Storing a bool costs more than comparing again, so each test of it gets its own copy of the comparison, made
/// right there, when what it compares is assigned only once (so it is the same at every test).
pub fn rematerialize(blocks: &mut [Block], locals: &mut Ordered<Local, Ty>) {
    let assigned = |blocks: &[Block], x: Local| -> usize {
        // `x = ...`, or `(x.k: T) = ...` and what is indexed off that field
        let by = |p: &Place| {
            p.base == PlaceBase::Local(x)
                && (p.projection.is_empty() || (matches!(p.projection[0], Projection::Field(..)) && p.wrapped() == 1))
        };
        blocks
            .iter()
            .map(|b| {
                b.statements
                    .iter()
                    .filter(|s| matches!(s, Statement::Assign(p, _) if by(p)))
                    .count()
                    + matches!(&b.terminator, Terminator::Call { dest, .. } if by(dest)) as usize
            })
            .sum()
    };
    // new locals past rustc's
    let mut fresh = 1 + locals
        .keys()
        .filter(|l| **l != Local::UNIT)
        .map(|l| l.0 as i64)
        .fold(0, i64::max);
    for bi in 0..blocks.len() {
        for s in blocks[bi].statements.clone() {
            let Statement::Assign(
                d,
                Rvalue::BinaryOp(op @ (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne), x, y),
            ) = &s
            else {
                continue;
            };
            let Some(b) = d.as_local().filter(|l| *l != Local::UNIT) else {
                continue;
            };
            // each side: a whole local, or a constant
            let side = |o: &Operand| -> Option<Operand> {
                match o {
                    Operand::Constant(c) if c.is_scalar() => Some(o.clone()),
                    _ => o
                        .local()
                        .filter(|l| *l != Local::UNIT)
                        .map(|l| Operand::Copy(Place::local(l))),
                }
            };
            let (Some(x), Some(y)) = (side(x), side(y)) else {
                continue;
            };
            let uses: Vec<usize> = (0..blocks.len())
                .filter(|&k| matches!(&blocks[k].terminator, Terminator::SwitchInt { discr: Operand::Copy(p) | Operand::Move(p), .. } if p.as_local() == Some(b)))
                .collect();
            let other = lines_naming(blocks, b) as i64 - 1 - uses.len() as i64;
            if uses.len() < 2 || other != 0 {
                continue;
            }
            if [&x, &y]
                .iter()
                .any(|v| v.local().is_some_and(|l| assigned(blocks, l) != 1))
            {
                continue;
            }
            let Some(at) = blocks[bi].statements.iter().position(|z| *z == s) else {
                continue;
            };
            blocks[bi].statements.remove(at);
            for &u in &uses {
                let n = Local(fresh as u32);
                fresh += 1;
                locals.set(n, Ty::Bool);
                blocks[u].statements.push(Statement::Assign(
                    Place::local(n),
                    Rvalue::BinaryOp(*op, x.clone(), y.clone()),
                ));
                if let Terminator::SwitchInt { discr, .. } = &mut blocks[u].terminator {
                    *discr = Operand::Move(Place::local(n));
                }
            }
        }
    }
}
