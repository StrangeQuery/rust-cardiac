//! Cards straight to and from arrays. CARDIAC can INP or OUT any cell, so `out(xs[i])` and `xs[i] = inp()` need no
//! copy: the instruction itself is built on the element's address (as hand-written code does). And an index that
//! is another index plus a constant, xs[i + 2], folds the constant into the address. Runs after the range pass, so
//! only indexes whose bounds checks were proven away are folded (a check still standing reads the index).
use super::ir::{
    blocks_naming, find, lines_naming as uses, BasicBlock, BinOp, Block, IntTy, Local, Locals, Operand, Place,
    PlaceBase, Projection, Rvalue, Statement, Terminator,
};

/// A place built on a computed address: xs[i], xs[i+k] or (*p)[i].
fn dyn_place(p: &Place) -> bool {
    matches!(p.base, PlaceBase::Local(_))
        && matches!(
            p.projection.as_slice(),
            [Projection::Index(_)] | [Projection::IndexPlus(..)] | [Projection::Deref, Projection::Index(_)]
        )
}

/// Where a call returns to, or a goto goes.
fn return_or_goto(t: &Terminator) -> Option<BasicBlock> {
    match t {
        Terminator::Call { target, .. } => *target,
        Terminator::Goto(b) => Some(*b),
        _ => None,
    }
}

/// `x = copy p;` for a whole local x: x and p.
fn copy_of(s: &Statement) -> Option<(Local, &Place)> {
    match s {
        Statement::Assign(d, Rvalue::Use(Operand::Copy(p))) => {
            d.as_local().filter(|l| *l != Local::UNIT).map(|l| (l, p))
        }
        _ => None,
    }
}

/// `x = inp() -> [return: bb]` for a whole local x: x and where it returns to.
fn card_read(t: &Terminator) -> Option<(Local, BasicBlock)> {
    let Terminator::Call {
        dest,
        func,
        args,
        target: Some(target),
        ..
    } = t
    else {
        return None;
    };
    if !func.is("inp") || !args.is_empty() {
        return None;
    }
    dest.as_local().filter(|l| *l != Local::UNIT).map(|l| (l, *target))
}

/// A place indexed by `k` straight off a local: `xs[k]`.
fn indexed_by(p: &Place, k: Local) -> bool {
    matches!(p.base, PlaceBase::Local(_)) && matches!(p.projection.first(), Some(Projection::Index(i)) if *i == k)
}

pub fn io_pass(blocks: &mut [Block]) {
    for bi in 0..blocks.len() {
        // xs[i + k]
        let mut i = 0;
        while i < blocks[bi].statements.len() {
            let Statement::Assign(d, Rvalue::BinaryOp(BinOp::Add, Operand::Copy(cp), konst)) =
                &blocks[bi].statements[i]
            else {
                i += 1;
                continue;
            };
            let (Some(k), Some(c), Some((n, _))) = (
                d.as_local(),
                cp.as_local(),
                konst.int().filter(|(v, t)| *t == IntTy::Usize && *v >= 0),
            ) else {
                i += 1;
                continue;
            };
            if k == Local::UNIT || c == Local::UNIT {
                i += 1;
                continue;
            }
            let b = &blocks[bi];
            let rest: Vec<&dyn Locals> = b.lines().skip(i + 1).collect();
            let reassigned = b.statements[i + 1..]
                .iter()
                .position(|s| matches!(s, Statement::Assign(p, _) if p.as_local() == Some(c)))
                .or_else(|| {
                    matches!(&b.terminator, Terminator::Call { dest, .. } if dest.as_local() == Some(c))
                        .then_some(b.statements.len() - i - 1)
                });
            let index_uses = |line_places: Vec<&Place>| line_places.into_iter().filter(|p| indexed_by(p, k)).count();
            let as_index = b.statements[i + 1..]
                .iter()
                .all(|s| s.count(k) == index_uses(s.places()))
                && b.terminator.count(k) == index_uses(b.terminator.places());
            let ok = uses(blocks, k) == 1 + rest.iter().filter(|l| l.mentions(k)).count()
                && as_index
                && reassigned.is_none_or(|r| !rest[r..].iter().any(|l| l.mentions(k)));
            if !ok {
                i += 1;
                continue;
            }
            let b = &mut blocks[bi];
            b.statements.remove(i);
            let fold = &mut |p: &mut Place| {
                if indexed_by(p, k) {
                    p.projection[0] = Projection::IndexPlus(c, n as u32);
                }
            };
            for s in b.statements[i..].iter_mut() {
                s.places_mut(fold);
            }
            b.terminator.places_mut(fold);
        }
        // out(xs[i])
        let b = &blocks[bi];
        if let (Terminator::Call { dest, func, args, .. }, Some(prev)) = (&b.terminator, b.statements.last()) {
            let whole = dest.as_local().is_some_and(|l| l != Local::UNIT);
            if let (true, [Operand::Move(arg)], Some((x, p))) =
                (whole && func.is("out"), args.as_slice(), copy_of(prev))
            {
                if arg.as_local() == Some(x) && dyn_place(p) && uses(blocks, x) == 2 {
                    let p = p.clone();
                    let b = &mut blocks[bi];
                    if let Terminator::Call { args, .. } = &mut b.terminator {
                        args[0] = Operand::Place(p);
                    }
                    b.statements.pop();
                }
            }
        }
    }
    // digit(xs[i], k): the element read straight into the accumulator, after the shift is built
    for bi in 0..blocks.len() {
        let b = &blocks[bi];
        let (Terminator::Call { func, args, .. }, Some(prev)) = (&b.terminator, b.statements.last()) else {
            continue;
        };
        let (true, [Operand::Move(arg), k], Some((x, p))) = (func.is("digit"), args.as_slice(), copy_of(prev)) else {
            continue;
        };
        if arg.as_local() == Some(x) && dyn_place(p) && uses(blocks, x) == 2 && !k.mentions(x) {
            let p = p.clone();
            let b = &mut blocks[bi];
            if let Terminator::Call { args, .. } = &mut b.terminator {
                args[0] = Operand::Copy(p);
            }
            b.statements.pop();
        }
    }
    // [inp(), inp(), ...]: each card read straight into its element
    for bi in 0..blocks.len() {
        let mut i = 0;
        while i < blocks[bi].statements.len() {
            let Statement::Assign(d, Rvalue::Array(items)) = &blocks[bi].statements[i] else {
                i += 1;
                continue;
            };
            let Some(a) = d.as_local().filter(|l| *l != Local::UNIT) else {
                i += 1;
                continue;
            };
            let xs: Option<Vec<Local>> = items
                .iter()
                .map(|o| if let Operand::Move(p) = o { p.as_local() } else { None })
                .collect();
            let Some(xs) = xs else {
                i += 1;
                continue;
            };
            let defs: Vec<Option<usize>> = xs
                .iter()
                .map(|x| {
                    blocks
                        .iter()
                        .position(|d| card_read(&d.terminator).is_some_and(|(y, _)| y == *x))
                })
                .collect();
            if defs.iter().any(|d| d.is_none()) || !xs.iter().all(|x| uses(blocks, *x) == 2) {
                i += 1;
                continue;
            }
            let len = xs.len() as u64;
            for (k, d) in defs.iter().enumerate() {
                if let Terminator::Call { dest, .. } = &mut blocks[d.unwrap()].terminator {
                    *dest = Place {
                        base: PlaceBase::Local(a),
                        projection: vec![Projection::ConstantIndex {
                            offset: k as u64,
                            min_length: len,
                        }],
                    };
                }
            }
            blocks[bi].statements.remove(i);
        }
    }
    // a card read now and stored into an element a few blocks on (a struct's fields read, then stored): read
    // straight into the element, if nothing in between touches that array or its index
    for bi in 0..blocks.len() {
        let Some((card, next)) = card_read(&blocks[bi].terminator) else {
            continue;
        };
        if uses(blocks, card) != 2 {
            continue;
        }
        let first = find(blocks, next);
        let mut s = first;
        let mut j: Option<usize> = None;
        let mut seen: Vec<&dyn Locals> = vec![];
        let mut seen_defs: Vec<Option<Local>> = vec![];
        while let Some(si) = s {
            if blocks_naming(blocks, blocks[si].name) != 1 || si == bi {
                break;
            }
            j = blocks[si].lines().position(|l| l.mentions(card));
            if j.is_some() {
                break;
            }
            seen.extend(blocks[si].lines());
            seen_defs.extend(blocks[si].defs());
            s = return_or_goto(&blocks[si].terminator).and_then(|n| find(blocks, n));
        }
        let (Some(si), Some(j)) = (s, j) else { continue };
        if Some(si) == first && j == 0 {
            continue; // the next rule's case
        }
        let Some(Statement::Assign(place, Rvalue::Use(Operand::Move(from)))) = blocks[si].statements.get(j) else {
            continue;
        };
        let (Some(array), [index]) = (place.base_local(), place.projection.as_slice()) else {
            continue;
        };
        let index_local = match index {
            Projection::Index(l) => Some(*l),
            Projection::ConstantIndex { .. } => None,
            _ => continue,
        };
        if from.as_local() != Some(card) || array == Local::UNIT {
            continue;
        }
        let before: Vec<&dyn Locals> = blocks[si].lines().take(j).collect();
        let before_defs: Vec<Option<Local>> = blocks[si].defs().into_iter().take(j).collect();
        let touches = seen.iter().chain(&before).any(|l| l.mentions(array))
            || index_local.is_some_and(|x| seen_defs.iter().chain(&before_defs).any(|d| *d == Some(x)));
        if touches {
            continue;
        }
        let place = place.clone();
        if let Terminator::Call { dest, .. } = &mut blocks[bi].terminator {
            *dest = place;
        }
        blocks[si].statements.remove(j);
    }
    // xs[i] = inp()
    for bi in 0..blocks.len() {
        let Some((card, next)) = card_read(&blocks[bi].terminator) else {
            continue;
        };
        let Some(si) = find(blocks, next) else { continue };
        let Some(Statement::Assign(place, Rvalue::Use(Operand::Move(from)))) = blocks[si].statements.first() else {
            continue;
        };
        if from.as_local() != Some(card) || !dyn_place(place) || uses(blocks, card) != 2 {
            continue;
        }
        if blocks_naming(blocks, blocks[si].name) != 1 {
            continue;
        }
        let place = place.clone();
        if let Terminator::Call { dest, .. } = &mut blocks[bi].terminator {
            *dest = place;
        }
        blocks[si].statements.remove(0);
    }
}
