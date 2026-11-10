//! Iterators over an array that rustc doesn't inline into code the compiler already knows. Each is a counted loop
//! over the array's indexes, so each becomes one: a `core::ops::Range<usize>` the for-loop pass then lowers (its
//! `next()` a test and a step), and the item worked out from the index.
//!   `for x in t`, an `array::IntoIter<T, N>`: its `alive` range, with `t[i]` the item (t itself, where nothing
//!     writes t again, else a copy taken where the iterator was made, as `IntoIter` takes one)
//! The iterator's own `drop` does nothing for elements a cell holds, and goes.
use std::collections::HashMap;

use super::ir::{
    self, ArrayLen, BasicBlock, BorrowKind, CastKind, Const, GenericArg, GenericArgs, IntTy, Local, Operand, Path,
    Place, PlaceBase, Projection, QSelf, Rvalue, Segment, Statement, Terminator, Ty, Unwind,
};
use super::structs::Function;

/// A path of plain names, the last with these type arguments.
pub(super) fn path(names: &[&str], args: Vec<Ty>) -> Path {
    let n = names.len();
    Path {
        qself: None,
        segments: names
            .iter()
            .enumerate()
            .map(|(k, name)| Segment::Name {
                name: name.to_string(),
                args: (k + 1 == n && !args.is_empty()).then(|| GenericArgs {
                    turbofish: false,
                    args: args.iter().cloned().map(GenericArg::Ty).collect(),
                }),
            })
            .collect(),
    }
}

/// `core::ops::Range<usize>`.
fn index_range() -> Ty {
    Ty::Path(path(&["core", "ops", "Range"], vec![Ty::Int(IntTy::Usize)]))
}

/// `core::option::Option<T>`.
pub(super) fn option(t: Ty) -> Ty {
    Ty::Path(path(&["core", "option", "Option"], vec![t]))
}

pub(super) fn usize_const(v: u64) -> Operand {
    Operand::Constant(Const::Int {
        value: v as i128,
        ty: IntTy::Usize,
    })
}

/// The names of a path's segments, if all are plain names (type arguments aside).
pub(super) fn names(p: &Path) -> Option<Vec<&str>> {
    p.segments
        .iter()
        .map(|s| match s {
            Segment::Name { name, .. } => Some(name.as_str()),
            Segment::Impl(_) => None,
        })
        .collect()
}

/// The last segment's type arguments.
pub(super) fn last_args(p: &Path) -> Vec<&GenericArg> {
    match p.segments.last() {
        Some(Segment::Name { args: Some(g), .. }) => g.args.iter().collect(),
        _ => vec![],
    }
}

/// `core::array::IntoIter<T, N>` for T a cell holds: T and N.
fn array_into_iter_ty(t: &Ty) -> Option<(Ty, u64)> {
    let Ty::Path(p) = t else { return None };
    if p.qself.is_some() || names(p)? != ["core", "array", "IntoIter"] {
        return None;
    }
    match last_args(p).as_slice() {
        [GenericArg::Ty(e @ (Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool)), GenericArg::Const(n)] => {
            Some((e.clone(), *n))
        }
        _ => None,
    }
}

/// One of the structs an `IntoIter` wraps its range and its array in, by its type: `ManuallyDrop`, `MaybeDangling`,
/// `PolymorphicIter` (of anything).
fn wrapper(t: &Ty) -> bool {
    let Ty::Path(p) = t else { return false };
    let Some(ns) = names(p) else { return false };
    p.qself.is_none()
        && matches!(
            ns.as_slice(),
            ["core", "mem", "ManuallyDrop"]
                | ["core", "mem", "MaybeDangling"]
                | ["core", "array", "iter", "iter_inner", "PolymorphicIter"]
        )
}

/// Where a local is assigned: a statement (block, line), or a call's destination (block).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum At {
    Line(usize, usize),
    Call(usize),
}

/// How an `IntoIter` was made: the lines that build it, which go, the one that becomes the range, the range, and
/// the array.
struct Made {
    gone: Vec<At>,
    /// the array's cast to `[MaybeUninit<T>; N]` (opt-level 2), which becomes the copy if one is needed
    cast: Option<At>,
    root: At,
    /// the locals that hold the IntoIter itself, which become the range
    held: Vec<Local>,
    start: u64,
    end: u64,
    array: Local,
}

/// The whole-local definitions of each local.
pub(super) fn definitions(f: &Function) -> HashMap<Local, Vec<At>> {
    let mut defs: HashMap<Local, Vec<At>> = HashMap::new();
    for (bi, b) in f.body.blocks.iter().enumerate() {
        for (si, s) in b.statements.iter().enumerate() {
            if let Some(l) = s.def() {
                defs.entry(l).or_default().push(At::Line(bi, si));
            }
        }
        if let Some(l) = b.terminator.def() {
            defs.entry(l).or_default().push(At::Call(bi));
        }
    }
    defs
}

/// Is the place written anywhere (assigned, its address taken to write through, a call's result), but at `except`?
fn writes(f: &Function, l: Local, except: At) -> usize {
    let mut n = 0;
    for (bi, b) in f.body.blocks.iter().enumerate() {
        for (si, s) in b.statements.iter().enumerate() {
            if let Statement::Assign(d, r) = s {
                let mutable =
                    matches!(r, Rvalue::Ref(BorrowKind::Mut | BorrowKind::RawMut, p) if p.base_local() == Some(l));
                if (d.base_local() == Some(l) || mutable) && At::Line(bi, si) != except {
                    n += 1;
                }
            }
        }
        match &b.terminator {
            Terminator::Call { dest, args, .. } => {
                let through = args
                    .iter()
                    .any(|a| matches!(a, Operand::Move(p) if p.base_local() == Some(l) && !p.projection.is_empty()));
                if (dest.base_local() == Some(l) || through) && At::Call(bi) != except {
                    n += 1;
                }
            }
            Terminator::Drop { place, .. } if place.base_local() == Some(l) => n += 1,
            _ => {}
        }
    }
    n
}

/// How the `IntoIter` in `it` was made, if as rustc makes one from a whole array: its `into_iter()` call (opt-level
/// 1), or the structs that call builds inlined (2). Each local between is read once, on the way.
fn made(f: &Function, it: Local, types: &HashMap<Local, Ty>, defs: &HashMap<Local, Vec<At>>) -> Option<Made> {
    let mut gone = vec![];
    let mut held = vec![];
    let mut root = None;
    let mut l = it;
    loop {
        let [at] = defs.get(&l)?.as_slice() else { return None };
        if l != it && ir::occurrences(&f.body.blocks, l) != 2 {
            return None;
        }
        let into_iter = array_into_iter_ty(types.get(&l)?).is_some();
        if into_iter {
            held.push(l);
        }
        let next = match *at {
            At::Call(bi) => {
                // `<[T; N] as IntoIterator>::into_iter(move t)`
                let Terminator::Call { func, args, .. } = &f.body.blocks[bi].terminator else {
                    return None;
                };
                let [Operand::Move(a) | Operand::Copy(a)] = args.as_slice() else {
                    return None;
                };
                let q = func.qself.as_ref()?;
                let is_trait = q
                    .as_trait
                    .as_ref()
                    .is_some_and(|t| names(t) == Some(vec!["IntoIterator"]));
                let (Ty::Array(_, ArrayLen::Known(n)), true, true, Some(a)) =
                    (&q.ty, is_trait && func.name() == "into_iter", into_iter, a.as_local())
                else {
                    return None;
                };
                return Some(Made {
                    gone,
                    cast: None,
                    root: *at,
                    held,
                    start: 0,
                    end: *n,
                    array: a,
                });
            }
            At::Line(bi, si) => {
                let Statement::Assign(_, r) = &f.body.blocks[bi].statements[si] else {
                    return None;
                };
                match r {
                    Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) if into_iter => p.as_local()?,
                    // `IntoIter { inner }`, `ManuallyDrop { value }`, `MaybeDangling(..)`
                    Rvalue::Struct(_, fs) if fs.len() == 1 && (into_iter || wrapper(types.get(&l)?)) => {
                        if into_iter {
                            root = Some(*at);
                        } else {
                            gone.push(*at);
                        }
                        fs[0].1.local()?
                    }
                    Rvalue::Variant(_, os) if os.len() == 1 && wrapper(types.get(&l)?) => {
                        gone.push(*at);
                        os[0].local()?
                    }
                    // `PolymorphicIter { alive: IndexRange { start, end }, data }`, data the array cast
                    Rvalue::Struct(_, fs) if wrapper(types.get(&l)?) => {
                        let [(alive, Operand::Constant(Const::Adt(_, range))), (data, d)] = fs.as_slice() else {
                            return None;
                        };
                        let bound = |name: &str| {
                            range.iter().find(|(k, _)| k == name).and_then(|(_, c)| match c {
                                Const::Int { value, .. } => u64::try_from(*value).ok(),
                                _ => None,
                            })
                        };
                        let (Some(start), Some(end)) = (bound("start"), bound("end")) else {
                            return None;
                        };
                        if alive != "alive" || data != "data" {
                            return None;
                        }
                        gone.push(*at);
                        let d = d.local()?;
                        let [cast @ At::Line(ci, cj)] = defs.get(&d)?.as_slice() else {
                            return None;
                        };
                        if ir::occurrences(&f.body.blocks, d) != 2 {
                            return None;
                        }
                        let Statement::Assign(_, Rvalue::Cast(a, _, CastKind::Transmute)) =
                            &f.body.blocks[*ci].statements[*cj]
                        else {
                            return None;
                        };
                        let a = a.local()?;
                        if !matches!(types.get(&a), Some(Ty::Array(_, ArrayLen::Known(_)))) {
                            return None;
                        }
                        return Some(Made {
                            gone,
                            cast: Some(*cast),
                            root: root?,
                            held,
                            start,
                            end,
                            array: a,
                        });
                    }
                    _ => return None,
                }
            }
        };
        l = next;
    }
}

/// A `next()` on the iterator: its block, the lines that go (a cast of the borrow), the `&mut` local it now
/// borrows the range through, and the item.
struct Next {
    block: usize,
    gone: Vec<usize>,
    borrow: Local,
    item: Local,
}

/// Each `next()` on `it`, which it borrows whole or (`inner`) through fields 0 of the structs it wraps, and each
/// drop of it: none if `it` is used in any other way.
fn nexts_of(f: &Function, it: Local, inner: bool) -> Option<(Vec<Next>, Vec<usize>)> {
    let mut nexts = vec![];
    let mut drops = vec![];
    let mut seen = 1; // its definition
    for (bi, b) in f.body.blocks.iter().enumerate() {
        seen += b
            .statements
            .iter()
            .filter(|s| matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if *l == it))
            .count();
        if matches!(&b.terminator, Terminator::Drop { place, .. } if place.as_local() == Some(it)) {
            drops.push(bi);
            seen += 1;
            continue;
        }
        let Terminator::Call { dest, func, args, .. } = &b.terminator else {
            continue;
        };
        let [Operand::Move(r)] = args.as_slice() else { continue };
        let (Some(item), Some(r)) = (dest.as_local(), r.as_local()) else {
            continue;
        };
        if func.name() != "next" {
            continue;
        }
        let borrowed = |l: Local| {
            b.statements.iter().any(|s| match s {
                Statement::Assign(d, Rvalue::Ref(BorrowKind::Mut, p)) => {
                    d.as_local() == Some(l)
                        && p.base_local() == Some(it)
                        && (p.projection.is_empty()
                            || inner && p.projection.iter().all(|x| matches!(x, Projection::Field(0, _))))
                }
                _ => false,
            })
        };
        let found = if borrowed(r) {
            Some((r, vec![]))
        } else {
            // `r = copy r2 as &mut PolymorphicIter<[MaybeUninit<T>]> (Unsize)`, a copy once `slices` has seen it
            b.statements.iter().enumerate().find_map(|(ci, s)| match s {
                Statement::Assign(
                    d,
                    Rvalue::Cast(Operand::Copy(q) | Operand::Move(q), _, CastKind::Unsize(_))
                    | Rvalue::Use(Operand::Copy(q) | Operand::Move(q)),
                ) if inner && d.as_local() == Some(r) => {
                    let r2 = q.as_local()?;
                    let once = ir::occurrences(&f.body.blocks, r) == 2 && ir::occurrences(&f.body.blocks, r2) == 2;
                    (borrowed(r2) && once).then(|| (r2, vec![ci]))
                }
                _ => None,
            })
        };
        let Some((borrow, gone)) = found else { continue };
        seen += 1;
        nexts.push(Next {
            block: bi,
            gone,
            borrow,
            item,
        });
    }
    (!nexts.is_empty() && seen == ir::occurrences(&f.body.blocks, it)).then_some((nexts, drops))
}

/// Is every use of the item its variant tested, its value read (`read` says which places read it), or its storage?
fn items_fit(f: &Function, nexts: &[Next], read: &dyn Fn(&Place) -> bool) -> bool {
    nexts.iter().all(|n| {
        let item = n.item;
        let mut uses = 1; // the call
        for s in f.body.blocks.iter().flat_map(|b| &b.statements) {
            uses += match s {
                Statement::StorageLive(l) | Statement::StorageDead(l) if *l == item => 1,
                Statement::Assign(d, Rvalue::Discriminant(p)) if p.as_local() == Some(item) => {
                    (d.base_local() != Some(item)) as usize
                }
                Statement::Assign(d, Rvalue::Use(Operand::Copy(p) | Operand::Move(p)))
                    if p.base_local() == Some(item) && read(p) =>
                {
                    (d.base_local() != Some(item)) as usize
                }
                _ => 0,
            };
        }
        uses == ir::occurrences(&f.body.blocks, item)
    })
}

/// The iterator in `held` (the last of them assigned at `root`, the rest copies of it) a `Range<usize>` from start
/// to end, each `next()` the range's, and each read of an item's value what `value` makes of the index: the
/// payload's projections after `(item as Some).0`, the index, and where to put it.
#[allow(clippy::too_many_arguments)]
fn as_range(
    f: &mut Function,
    held: &[Local],
    root: At,
    start: Operand,
    end: Operand,
    nexts: &[Next],
    drops: &[usize],
    value: &dyn Fn(&[Projection], Local, Place) -> Vec<Statement>,
) {
    let mut next_local = f
        .body
        .decls
        .iter()
        .map(|d| d.local.0)
        .chain(f.params.iter().map(|(l, _)| l.0))
        .max()
        .unwrap_or(0)
        + 1;
    let range = Statement::Assign(
        Place::local(held[held.len() - 1]),
        Rvalue::Struct(
            path(&["core", "ops", "Range"], vec![Ty::Int(IntTy::Usize)]),
            vec![("start".into(), start), ("end".into(), end)],
        ),
    );
    match root {
        At::Line(bi, si) => f.body.blocks[bi].statements[si] = range,
        At::Call(bi) => {
            let b = &mut f.body.blocks[bi];
            let Terminator::Call { target: Some(t), .. } = b.terminator else {
                unreachable!("an iterator made by a call that returns")
            };
            b.statements.push(range);
            b.terminator = Terminator::Goto(t);
        }
    }
    let next_path = Path {
        qself: Some(Box::new(QSelf {
            ty: index_range(),
            as_trait: Some(path(&["Iterator"], vec![])),
        })),
        segments: vec![Segment::Name {
            name: "next".into(),
            args: None,
        }],
    };
    let it = held[0];
    let mut gone_locals: Vec<Local> = vec![];
    for n in nexts {
        let b = &mut f.body.blocks[n.block];
        for ci in &n.gone {
            gone_locals.extend(b.statements[*ci].def());
            b.statements[*ci] = Statement::Nop;
        }
        for s in b.statements.iter_mut() {
            if let Statement::Assign(d, Rvalue::Ref(BorrowKind::Mut, p)) = s {
                if d.as_local() == Some(n.borrow) {
                    *p = Place::local(it);
                }
            }
        }
        if let Terminator::Call { func, args, .. } = &mut b.terminator {
            *func = next_path.clone();
            *args = vec![Operand::Move(Place::local(n.borrow))];
        }
    }
    // each read of an item's value: its index, then what the iterator gives for it
    let items: Vec<Local> = nexts.iter().map(|n| n.item).collect();
    let mut fresh = vec![];
    for b in f.body.blocks.iter_mut() {
        let mut out = Vec::with_capacity(b.statements.len());
        for s in std::mem::take(&mut b.statements) {
            let read = match &s {
                Statement::Assign(d, Rvalue::Use(Operand::Copy(p) | Operand::Move(p)))
                    if p.base_local().is_some_and(|l| items.contains(&l)) && p.projection.len() >= 2 =>
                {
                    Some((d.clone(), p.clone()))
                }
                _ => None,
            };
            let Some((d, p)) = read else {
                if s != Statement::Nop {
                    out.push(s);
                }
                continue;
            };
            let i = Local(next_local);
            next_local += 1;
            fresh.push(i);
            out.push(Statement::Assign(
                Place::local(i),
                Rvalue::Use(Operand::Copy(Place {
                    base: p.base.clone(),
                    projection: vec![
                        Projection::Downcast("Some".into()),
                        Projection::Field(0, Ty::Int(IntTy::Usize)),
                    ],
                })),
            ));
            out.extend(value(&p.projection[2..], i, d));
        }
        b.statements = out;
    }
    for &bi in drops {
        if let Terminator::Drop { target, .. } = f.body.blocks[bi].terminator {
            f.body.blocks[bi].terminator = Terminator::Goto(target);
        }
    }
    // While the iterator lives, a call that panics unwinds into a cleanup block that drops it (the IR leaves
    // cleanup blocks out). CARDIAC never unwinds, and that drop does nothing, so the unwind is a plain one: any
    // other value whose drop did something is dropped on the way out too, and that drop is refused.
    let names: Vec<BasicBlock> = f.body.blocks.iter().map(|b| b.name).collect();
    // (a check that panics too: `for off in T { n += b[s + off] }` checks the index inside the loop)
    for b in f.body.blocks.iter_mut() {
        if let Terminator::Call { unwind, .. } | Terminator::Assert { unwind, .. } = &mut b.terminator {
            if matches!(unwind, Unwind::Cleanup(c) if !names.contains(c)) {
                *unwind = Unwind::Continue;
            }
        }
    }
    // the types: the range where the iterator was, an index where its item was
    let borrows: Vec<Local> = nexts.iter().map(|n| n.borrow).collect();
    for d in f.body.decls.iter_mut() {
        if held.contains(&d.local) {
            d.ty = index_range();
        } else if items.contains(&d.local) {
            d.ty = option(Ty::Int(IntTy::Usize));
        } else if borrows.contains(&d.local) {
            d.ty = Ty::Ref {
                lifetime: None,
                mutable: true,
                to: Box::new(index_range()),
            };
        }
    }
    f.body.decls.extend(fresh.into_iter().map(|i| ir::Decl {
        local: i,
        mutable: false,
        ty: Ty::Int(IntTy::Usize),
    }));
    let blocks = &f.body.blocks;
    f.body
        .decls
        .retain(|d| !gone_locals.contains(&d.local) || ir::occurrences(blocks, d.local) > 0);
}

/// `for x in t` over an array by value, as a range over its indexes.
pub fn array_into_iter(f: &mut Function) {
    let iters: Vec<(Local, Ty)> = f
        .body
        .decls
        .iter()
        .filter_map(|d| array_into_iter_ty(&d.ty).map(|(e, _)| (d.local, e)))
        .collect();
    for (it, elem) in iters {
        let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
        types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
        let defs = definitions(f);
        if defs.get(&it).map(Vec::len) != Some(1) {
            continue;
        }
        let Some(m) = made(f, it, &types, &defs) else { continue };
        let Some((nexts, drops)) = nexts_of(f, it, true) else {
            continue;
        };
        let read = |p: &Place| matches!(p.projection.as_slice(), [Projection::Downcast(v), Projection::Field(0, t)] if v == "Some" && *t == elem);
        if !items_fit(f, &nexts, &read) {
            continue;
        }
        // the elements: the array itself, where nothing writes it again; else a copy, as IntoIter has
        let source = m.array;
        let alias = !f.params.iter().any(|(p, _)| *p == source)
            && matches!(defs.get(&source).map(Vec::as_slice), Some([d]) if writes(f, source, *d) == 0);
        let data = if alias {
            source
        } else {
            let d = Local(types.keys().map(|l| l.0).max().unwrap_or(0) + 1);
            f.body.decls.push(ir::Decl {
                local: d,
                mutable: false,
                ty: types[&source].clone(),
            });
            d
        };
        let copy_data = Statement::Assign(Place::local(data), Rvalue::Use(Operand::Copy(Place::local(source))));
        // the structs around the range gone, and the copy where the array was cast to make one
        let mut gone: Vec<Local> = vec![];
        for at in m.gone.iter().chain(&m.cast) {
            if let At::Line(bi, si) = *at {
                gone.extend(f.body.blocks[bi].statements[si].def());
                f.body.blocks[bi].statements[si] = Statement::Nop;
            }
        }
        match m.cast {
            Some(At::Line(bi, si)) => {
                f.body.blocks[bi].statements[si] = if alias { Statement::Nop } else { copy_data };
            }
            _ if !alias => {
                let At::Call(bi) = m.root else { continue };
                f.body.blocks[bi].statements.push(copy_data);
            }
            _ => {}
        }
        let element = |_: &[Projection], i: Local, d: Place| {
            vec![Statement::Assign(
                d,
                Rvalue::Use(Operand::Copy(Place {
                    base: PlaceBase::Local(data),
                    projection: vec![Projection::Index(i)],
                })),
            )]
        };
        as_range(
            f,
            &m.held,
            m.root,
            usize_const(m.start),
            usize_const(m.end),
            &nexts,
            &drops,
            &element,
        );
        // the structs' locals, now named nowhere
        let blocks = &f.body.blocks;
        f.body
            .decls
            .retain(|d| !gone.contains(&d.local) || ir::occurrences(blocks, d.local) > 0);
    }
}

/// Where a pointer into an array points: the array (a local, or behind a parameter that never changes) and the
/// element. Followed through the copies, casts and steps `slices` leaves of a slice iterator's pointers.
pub(super) fn element_pointer(f: &Function, defs: &HashMap<Local, Vec<At>>, o: &Operand) -> Option<(Place, u64)> {
    let l = o.local()?;
    let Some(ds) = defs.get(&l) else {
        // a parameter's own array, which nothing assigns
        let param = f.params.iter().find(|(p, _)| *p == l)?;
        return matches!(&param.1, Ty::Ref { to, .. } if matches!(**to, Ty::Array(..))).then(|| {
            let deref = Place {
                base: PlaceBase::Local(l),
                projection: vec![Projection::Deref],
            };
            (deref, 0)
        });
    };
    let [At::Line(bi, si)] = ds.as_slice() else { return None };
    let Statement::Assign(_, r) = &f.body.blocks[*bi].statements[*si] else {
        return None;
    };
    match r {
        Rvalue::Use(o) => element_pointer(f, defs, o),
        // (an `Offset` by an `isize` may go back)
        Rvalue::BinaryOp(ir::BinOp::Add, o, Operand::Constant(Const::Int { value, .. })) => {
            let (a, k) = element_pointer(f, defs, o)?;
            Some((a, u64::try_from(k as i128 + *value).ok()?))
        }
        Rvalue::Ref(_, p) => {
            let array = |t: Option<&Ty>| matches!(t, Some(Ty::Array(_, ArrayLen::Known(_))));
            let a = p.base_local()?;
            let ty = f.body.decls.iter().find(|d| d.local == a).map(|d| &d.ty);
            match p.projection.as_slice() {
                [] if array(ty) => Some((Place::local(a), 0)),
                [Projection::ConstantIndex { offset, .. }] if array(ty) => Some((Place::local(a), *offset)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `core::iter::Enumerate<I>`, for I one of `slices`' slice iterators.
fn enumerate_of_slice(t: &Ty) -> bool {
    let Ty::Path(p) = t else { return false };
    let [GenericArg::Ty(Ty::Path(i))] = last_args(p).as_slice() else {
        return false;
    };
    p.qself.is_none()
        && names(p).as_deref() == Some(&["core", "iter", "Enumerate"])
        && i.is_plain()
        && i.name().starts_with("slice_Iter")
}

/// `for (i, x) in t.iter().enumerate()`: a range over the array's indexes, with i the index (less where the slice
/// starts) and x the element's address.
pub fn enumerate_slices(f: &mut Function) {
    let iters: Vec<Local> = f
        .body
        .decls
        .iter()
        .filter(|d| enumerate_of_slice(&d.ty))
        .map(|d| d.local)
        .collect();
    for &it in &iters {
        let defs = definitions(f);
        // the copies it was made through, back to the literal
        let mut held = vec![it];
        let root = loop {
            let l = held[held.len() - 1];
            let Some([At::Line(bi, si)]) = defs.get(&l).map(Vec::as_slice) else {
                break None;
            };
            if l != it && ir::occurrences(&f.body.blocks, l) != 2 {
                break None;
            }
            match &f.body.blocks[*bi].statements[*si] {
                Statement::Assign(_, Rvalue::Use(Operand::Copy(q) | Operand::Move(q)))
                    if q.as_local().is_some_and(|q| iters.contains(&q)) =>
                {
                    held.push(q.as_local().unwrap())
                }
                Statement::Assign(_, Rvalue::Struct(_, fs)) => break Some((At::Line(*bi, *si), fs.clone())),
                _ => break None,
            }
        };
        let Some((root, fs)) = root else { continue };
        let [(iter, Operand::Copy(s) | Operand::Move(s)), (count, zero)] = fs.as_slice() else {
            continue;
        };
        let Some(s) = s.as_local() else { continue };
        if iter != "iter" || count != "count" || !matches!(zero.int(), Some((0, _))) {
            continue;
        }
        // the slice iterator inside: its two pointers, into one array
        let Some([At::Line(bi, si)]) = defs.get(&s).map(Vec::as_slice) else {
            continue;
        };
        if ir::occurrences(&f.body.blocks, s) != 2 {
            continue;
        }
        let Statement::Assign(_, Rvalue::Struct(_, sf)) = &f.body.blocks[*bi].statements[*si] else {
            continue;
        };
        let [(_, ptr), (_, end)] = sf.as_slice() else { continue };
        let (Some((a, from)), Some((b, to))) = (element_pointer(f, &defs, ptr), element_pointer(f, &defs, end)) else {
            continue;
        };
        if a != b || to < from {
            continue;
        }
        let Some((nexts, drops)) = nexts_of(f, it, false) else {
            continue;
        };
        let read = |p: &Place| matches!(p.projection.as_slice(), [Projection::Downcast(v), Projection::Field(0, Ty::Tuple(_)), Projection::Field(0 | 1, _)] if v == "Some");
        if !items_fit(f, &nexts, &read) {
            continue;
        }
        f.body.blocks[*bi].statements[*si] = Statement::Nop;
        let pair = |rest: &[Projection], i: Local, d: Place| {
            let index = Operand::Copy(Place::local(i));
            let r = match rest {
                [Projection::Field(0, _)] if from == 0 => Rvalue::Use(index),
                [Projection::Field(0, _)] => Rvalue::BinaryOp(ir::BinOp::Sub, index, usize_const(from)),
                _ => {
                    let mut at = a.clone();
                    at.projection.push(Projection::Index(i));
                    Rvalue::Ref(BorrowKind::Shared, at)
                }
            };
            vec![Statement::Assign(d, r)]
        };
        as_range(
            f,
            &held,
            root,
            usize_const(from),
            usize_const(to),
            &nexts,
            &drops,
            &pair,
        );
    }
}

/// What an iterator a method consumes runs over: indexes from `lo` up to `hi`, of type `ty`, each item an
/// element's address (`&t[i]`, the array's place given) or the index itself (a range's).
struct Over {
    lo: Operand,
    hi: Operand,
    ty: IntTy,
    elements: Option<Place>,
    /// the lines that made the iterator and borrowed it, which go
    made: Vec<(usize, usize)>,
}

/// The iterator a method is called on (`move s`, or `move r` with `r = &mut s` in the call's block): a slice
/// iterator into one array, or a range of a card's type built in the call's block from values nothing changes
/// before the call. Its local is used only there.
fn over(f: &Function, defs: &HashMap<Local, Vec<At>>, call: usize, recv: &Operand) -> Option<Over> {
    let b = &f.body.blocks[call];
    let mut s = recv.local()?;
    let mut made = vec![];
    let borrowed = b.statements.iter().enumerate().find_map(|(k, st)| match st {
        Statement::Assign(d, Rvalue::Ref(BorrowKind::Mut, p)) if d.as_local() == Some(s) && p.projection.is_empty() => {
            p.base_local().map(|t| (k, t))
        }
        _ => None,
    });
    if let Some((k, t)) = borrowed {
        made.push((call, k));
        if ir::occurrences(&f.body.blocks, s) != 2 {
            return None;
        }
        s = t;
    }
    if ir::occurrences(&f.body.blocks, s) != 2 {
        return None;
    }
    let [At::Line(bi, si)] = defs.get(&s)?.as_slice() else {
        return None;
    };
    let Statement::Assign(_, Rvalue::Struct(p, fs)) = &f.body.blocks[*bi].statements[*si] else {
        return None;
    };
    made.push((*bi, *si));
    let ty = f.body.decls.iter().find(|d| d.local == s).map(|d| &d.ty)?;
    if let Ty::Path(t) = ty {
        if t.is_plain() && t.name().starts_with("slice_Iter") {
            let [(_, ptr), (_, end)] = fs.as_slice() else {
                return None;
            };
            let (a, from) = element_pointer(f, defs, ptr)?;
            let (b, to) = element_pointer(f, defs, end)?;
            return (a == b && from <= to).then(|| Over {
                lo: usize_const(from),
                hi: usize_const(to),
                ty: IntTy::Usize,
                elements: Some(a),
                made,
            });
        }
    }
    // a range, `Range_T { start, end }` as the for-loop pass names it, in the call's block, its values unchanged
    // until the call
    let t = match p.is_plain().then(|| p.name()) {
        Some("Range_i16") => IntTy::I16,
        Some("Range_u8") => IntTy::U8,
        Some("Range_usize") => IntTy::Usize,
        _ => return None,
    };
    let [(st, lo), (en, hi)] = fs.as_slice() else {
        return None;
    };
    if st != "start" || en != "end" || *bi != call {
        return None;
    }
    let read = |o: &Operand| o.local();
    let changed = b.statements[si + 1..]
        .iter()
        .filter_map(|x| x.def())
        .any(|d| Some(d) == read(lo) || Some(d) == read(hi));
    if changed
        || ![lo, hi]
            .iter()
            .all(|o| matches!(o, Operand::Constant(_)) || o.local().is_some())
    {
        return None;
    }
    let copy = |o: &Operand| match o {
        Operand::Move(p) => Operand::Copy(p.clone()),
        o => o.clone(),
    };
    Some(Over {
        lo: copy(lo),
        hi: copy(hi),
        ty: t,
        elements: None,
        made,
    })
}

/// The iterator's own lines, gone (as `Nop`s, so the lines after keep their places until the pass is done).
fn gone(f: &mut Function, lines: &[(usize, usize)]) {
    for &(bi, si) in lines {
        f.body.blocks[bi].statements[si] = Statement::Nop;
    }
}

/// Fresh locals and blocks for the code a method becomes.
#[derive(Clone)]
pub(super) struct Fresh {
    local: u32,
    block: u32,
    pub(super) decls: Vec<ir::Decl>,
    pub(super) blocks: Vec<ir::Block>,
}

impl Fresh {
    pub(super) fn of(f: &Function) -> Fresh {
        let local = f
            .body
            .decls
            .iter()
            .map(|d| d.local.0)
            .chain(f.params.iter().map(|(l, _)| l.0))
            .max()
            .unwrap_or(0);
        let block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0);
        Fresh {
            local: local + 1,
            block: block + 1,
            decls: vec![],
            blocks: vec![],
        }
    }

    pub(super) fn local(&mut self, ty: Ty) -> Local {
        let l = Local(self.local);
        self.local += 1;
        self.decls.push(ir::Decl {
            local: l,
            mutable: true,
            ty,
        });
        l
    }

    pub(super) fn name(&mut self) -> BasicBlock {
        let b = BasicBlock(self.block);
        self.block += 1;
        b
    }

    pub(super) fn block(&mut self, name: BasicBlock, statements: Vec<Statement>, terminator: Terminator) {
        self.blocks.push(ir::Block {
            name,
            statements,
            terminator,
        });
    }
}

pub(super) fn assign(l: Local, r: Rvalue) -> Statement {
    Statement::Assign(Place::local(l), r)
}

pub(super) fn copy_of(l: Local) -> Operand {
    Operand::Copy(Place::local(l))
}

pub(super) fn branch(c: Local, zero: BasicBlock, other: BasicBlock) -> Terminator {
    Terminator::SwitchInt {
        discr: Operand::Move(Place::local(c)),
        arms: vec![(0, zero)],
        otherwise: Some(other),
    }
}

/// A call of a function by its name, returning to `target`.
pub(super) fn call(dest: Place, callee: &str, args: Vec<Operand>, target: BasicBlock) -> Terminator {
    Terminator::Call {
        dest,
        func: Path {
            qself: None,
            segments: vec![Segment::Name {
                name: callee.to_string(),
                args: None,
            }],
        },
        args,
        target: Some(target),
        unwind: Unwind::Continue,
    }
}

/// `Option::<T>::Some(v)` or `Option::<T>::None`.
pub(super) fn variant(t: &Ty, some: Option<Operand>) -> Rvalue {
    let segments = vec![
        Segment::Name {
            name: "Option".into(),
            args: Some(GenericArgs {
                turbofish: true,
                args: vec![GenericArg::Ty(t.clone())],
            }),
        },
        Segment::Name {
            name: if some.is_some() { "Some" } else { "None" }.into(),
            args: None,
        },
    ];
    Rvalue::Variant(Path { qself: None, segments }, some.into_iter().collect())
}

/// `reduce` with the closure core's `max` gives it (true) or `min`'s (false), on references to a card's type:
/// `{closure@core::iter::Iterator::max_by::fold<&i16, .. {<&i16 as Ord>::cmp}>::{closure#0}}`.
fn extreme(func: &Path) -> Option<bool> {
    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
        return None;
    };
    let Some(GenericArg::Ty(Ty::Other(c))) = g.args.last() else {
        return None;
    };
    let by = |m: &str| {
        ["i16", "u8", "i8", "usize"].iter().any(|t| {
            c.starts_with(&format!("{{closure@core::iter::Iterator::{m}_by::fold<&{t}, "))
                && c.ends_with(&format!("{{<&{t} as Ord>::cmp}}>::{{closure#0}}}}"))
        })
    };
    if by("max") {
        Some(true)
    } else if by("min") {
        Some(false)
    } else {
        None
    }
}

/// A path's last segment's last type argument.
fn last_arg_of(p: &Path) -> Option<&GenericArg> {
    match p.segments.last() {
        Some(Segment::Name { args: Some(g), .. }) => g.args.last(),
        _ => None,
    }
}

/// The closure a method is given, by its type among the method's type arguments: its function, which captures
/// nothing (the closure passed is `()`), or a closure of core's that sums.
enum Closure {
    Fn(String),
    Sum,
}

fn closure_of(func: &Path, arg: &Operand, closures: &HashMap<String, String>) -> Option<Closure> {
    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
        return None;
    };
    let Some(GenericArg::Ty(Ty::Other(c))) = g.args.last() else {
        return None;
    };
    // `{closure@<i16 as Sum<&i16>>::sum<..>::{closure#0}}`, passed as a zero-sized constant
    if c.starts_with("{closure@<i16 as Sum") || c.starts_with("{closure@<u8 as Sum") {
        return Some(Closure::Sum);
    }
    matches!(arg, Operand::Constant(Const::ZeroSized(_))).then(|| closures.get(c).cloned().map(Closure::Fn))?
}

/// Methods of an iterator over an array or a range that rustc calls rather than inlines, as the loops they are:
/// `fold` (and `sum`, a fold with core's closure), `all`, `any` and `position` with a closure that captures nothing,
/// and a range's `count`, `last` and `nth`. A closure is called as the closure's function is elsewhere; core's
/// sum is an overflow-checked add, so a sum past a card panics (natively it panics only past the type's range, if
/// overflow checks are on).
pub fn iterator_methods(f: &mut Function, closures: &HashMap<String, String>) {
    let mut fresh = Fresh::of(f);
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let Some(q) = &func.qself else { continue };
        if q.as_trait
            .as_ref()
            .and_then(names)
            .is_none_or(|t| t != ["Iterator"] && t != ["iter", "range", "RangeIteratorImpl"])
        {
            continue;
        }
        let Some(recv) = args.first() else { continue };
        let defs = definitions(f);
        let Some(o) = over(f, &defs, bi, recv) else { continue };
        let Some(d) = dest.as_local() else { continue };
        let it = Ty::Int(o.ty);
        let set = |v: Rvalue| Statement::Assign(dest.clone(), v);
        // without a loop: a slice's count, its length; a range's last, its end less one if it isn't empty
        match (func.name(), args.len(), &o.elements) {
            ("count", 1, Some(_)) => {
                let (Some((lo, _)), Some((hi, _))) = (o.lo.int(), o.hi.int()) else {
                    continue;
                };
                gone(f, &o.made);
                let b = &mut f.body.blocks[bi];
                b.statements.push(set(Rvalue::Use(usize_const((hi - lo) as u64))));
                b.terminator = Terminator::Goto(ret);
                continue;
            }
            ("last", 1, None) => {
                let some = fresh.name();
                let v = fresh.local(it.clone());
                let c = fresh.local(Ty::Bool);
                let one = Operand::Constant(Const::Int { value: 1, ty: o.ty });
                fresh.block(
                    some,
                    vec![
                        assign(v, Rvalue::BinaryOp(ir::BinOp::Sub, o.hi.clone(), one)),
                        set(variant(&it, Some(Operand::Move(Place::local(v))))),
                    ],
                    Terminator::Goto(ret),
                );
                let none = fresh.name();
                fresh.block(none, vec![set(variant(&it, None))], Terminator::Goto(ret));
                gone(f, &o.made);
                let b = &mut f.body.blocks[bi];
                b.statements
                    .push(assign(c, Rvalue::BinaryOp(ir::BinOp::Lt, o.lo.clone(), o.hi.clone())));
                b.terminator = branch(c, none, some);
                continue;
            }
            _ => {}
        }
        // the loop: i from lo while i < hi, the body at `body`, the step at `step`, out at `done`
        let i = fresh.local(it.clone());
        let (head, body, step) = (fresh.name(), fresh.name(), fresh.name());
        let c = fresh.local(Ty::Bool);
        let mut entry = vec![assign(i, Rvalue::Use(o.lo.clone()))];
        let element = |p: &Place| {
            let mut e = p.clone();
            e.projection.push(Projection::Index(i));
            e
        };
        let item = |fresh: &mut Fresh| -> (Local, Statement) {
            match &o.elements {
                Some(a) => {
                    let elem = match &types.get(&a.base_local().unwrap()) {
                        Some(Ty::Array(e, _)) => (**e).clone(),
                        Some(Ty::Ref { to, .. }) => match &**to {
                            Ty::Array(e, _) => (**e).clone(),
                            _ => Ty::Int(IntTy::I16),
                        },
                        _ => Ty::Int(IntTy::I16),
                    };
                    let x = fresh.local(Ty::Ref {
                        lifetime: None,
                        mutable: false,
                        to: Box::new(elem),
                    });
                    (x, assign(x, Rvalue::Ref(BorrowKind::Shared, element(a))))
                }
                None => {
                    let x = fresh.local(Ty::Int(o.ty));
                    (x, assign(x, Rvalue::Use(copy_of(i))))
                }
            }
        };
        let mut steps = vec![assign(
            i,
            Rvalue::BinaryOp(
                ir::BinOp::Add,
                copy_of(i),
                Operand::Constant(Const::Int { value: 1, ty: o.ty }),
            ),
        )];
        let done_block = |fresh: &mut Fresh, statements: Vec<Statement>| {
            let b = fresh.name();
            fresh.block(b, statements, Terminator::Goto(ret));
            b
        };
        let bool_const = |b: bool| Rvalue::Use(Operand::Constant(Const::Bool(b)));
        let method = func.name();
        let looped = match (method, args.as_slice()) {
            (m @ ("fold" | "sum"), [_, rest @ ..]) => {
                // `sum::<T>()`, where rustc calls it (opt-level 1), is a fold from 0 with core's closure
                let acc_ty = types.get(&d).cloned().unwrap_or(Ty::Int(IntTy::I16));
                let (init, cl) = match (m, rest) {
                    ("fold", [init, cl]) => {
                        let Some(cl) = closure_of(&func, cl, closures) else {
                            continue;
                        };
                        (init.clone(), cl)
                    }
                    ("sum", []) => {
                        let Ty::Int(t) = acc_ty else { continue };
                        (Operand::Constant(Const::Int { value: 0, ty: t }), Closure::Sum)
                    }
                    _ => continue,
                };
                let init = &init;
                let acc = fresh.local(acc_ty.clone());
                entry.push(assign(acc, Rvalue::Use(init.clone())));
                let after = fresh.name();
                match cl {
                    Closure::Fn(callee) => {
                        let (x, made) = item(&mut fresh);
                        let r = fresh.local(acc_ty.clone());
                        fresh.block(
                            body,
                            vec![made],
                            call(
                                Place::local(r),
                                &callee,
                                vec![Operand::Move(Place::local(acc)), Operand::Move(Place::local(x))],
                                after,
                            ),
                        );
                        steps.insert(0, assign(acc, Rvalue::Use(Operand::Move(Place::local(r)))));
                        fresh.block(after, vec![], Terminator::Goto(step));
                    }
                    Closure::Sum => {
                        let Ty::Int(t) = acc_ty else { continue };
                        let v = fresh.local(Ty::Int(t));
                        let read = match &o.elements {
                            Some(a) => Rvalue::Use(Operand::Copy(element(a))),
                            None => Rvalue::Use(copy_of(i)),
                        };
                        let pair = fresh.local(Ty::Tuple(vec![Ty::Int(t), Ty::Bool]));
                        fresh.block(
                            body,
                            vec![
                                assign(v, read),
                                assign(
                                    pair,
                                    Rvalue::BinaryOp(ir::BinOp::AddWithOverflow, copy_of(acc), copy_of(v)),
                                ),
                            ],
                            Terminator::Assert {
                                cond: Operand::Move(Place::field(pair, 1, Ty::Bool)),
                                expected: false,
                                msg: "attempt to compute `{} + {}`, which would overflow".into(),
                                msg_args: vec![copy_of(acc), copy_of(v)],
                                target: after,
                                unwind: Unwind::Continue,
                            },
                        );
                        fresh.block(
                            after,
                            vec![assign(
                                acc,
                                Rvalue::Use(Operand::Move(Place::field(pair, 0, Ty::Int(t)))),
                            )],
                            Terminator::Goto(step),
                        );
                    }
                }
                Some(done_block(
                    &mut fresh,
                    vec![set(Rvalue::Use(Operand::Move(Place::local(acc))))],
                ))
            }
            (m @ ("all" | "any" | "position"), [_, cl]) => {
                let Some(Closure::Fn(callee)) = closure_of(&func, cl, closures) else {
                    continue;
                };
                if m == "position" && o.elements.is_none() {
                    continue;
                }
                let (x, made) = item(&mut fresh);
                let r = fresh.local(Ty::Bool);
                let tested = fresh.name();
                fresh.block(
                    body,
                    vec![made],
                    call(Place::local(r), &callee, vec![Operand::Move(Place::local(x))], tested),
                );
                let (hit, miss) = match m {
                    "all" => (vec![set(bool_const(false))], vec![set(bool_const(true))]),
                    "any" => (vec![set(bool_const(true))], vec![set(bool_const(false))]),
                    _ => {
                        let k = fresh.local(Ty::Int(IntTy::Usize));
                        let at = Rvalue::BinaryOp(ir::BinOp::Sub, copy_of(i), o.lo.clone());
                        (
                            vec![
                                assign(k, at),
                                set(variant(&Ty::Int(IntTy::Usize), Some(Operand::Move(Place::local(k))))),
                            ],
                            vec![set(variant(&Ty::Int(IntTy::Usize), None))],
                        )
                    }
                };
                let hit = done_block(&mut fresh, hit);
                // all: stop at the first false; any and position: at the first true
                let t = if m == "all" {
                    branch(r, hit, step)
                } else {
                    branch(r, step, hit)
                };
                fresh.block(tested, vec![], t);
                Some(done_block(&mut fresh, miss))
            }
            // `find` on a slice's iterator: the closure is given a reference to the item, itself a reference
            ("find", [_, cl]) if o.elements.is_some() => {
                let Some(Closure::Fn(callee)) = closure_of(&func, cl, closures) else {
                    continue;
                };
                let (x, made) = item(&mut fresh);
                let Some(x_ty) = fresh.decls.iter().find(|d| d.local == x).map(|d| d.ty.clone()) else {
                    continue;
                };
                let rx = fresh.local(Ty::Ref {
                    lifetime: None,
                    mutable: false,
                    to: Box::new(x_ty.clone()),
                });
                let r = fresh.local(Ty::Bool);
                let tested = fresh.name();
                fresh.block(
                    body,
                    vec![made, assign(rx, Rvalue::Ref(BorrowKind::Shared, Place::local(x)))],
                    call(Place::local(r), &callee, vec![Operand::Move(Place::local(rx))], tested),
                );
                let hit = done_block(&mut fresh, vec![set(variant(&x_ty, Some(copy_of(x))))]);
                fresh.block(tested, vec![], branch(r, step, hit));
                Some(done_block(&mut fresh, vec![set(variant(&x_ty, None))]))
            }
            // `max()` and `min()` on a slice's iterator, a reduce with core's closure that keeps the greater (the
            // later of equals) or the lesser (the earlier): the index of the one kept, and a reference to it
            ("reduce", [_, Operand::Constant(Const::ZeroSized(_))]) if o.elements.is_some() && extreme(&func).is_some() => {
                let a = o.elements.as_ref().unwrap();
                let max = extreme(&func) == Some(true);
                let Some(Ty::Path(option)) = types.get(&d) else { continue };
                let Some(GenericArg::Ty(r @ Ty::Ref { .. })) = last_arg_of(option) else {
                    continue;
                };
                let Ty::Ref { to: elem, .. } = r else { continue };
                let (seen, k) = (fresh.local(Ty::Bool), fresh.local(Ty::Int(IntTy::Usize)));
                entry.push(assign(seen, bool_const(false)));
                entry.push(assign(k, Rvalue::Use(usize_const(0))));
                let at = |l: Local| {
                    let mut e = a.clone();
                    e.projection.push(Projection::Index(l));
                    e
                };
                let (first, compare, keep) = (fresh.name(), fresh.name(), fresh.name());
                fresh.block(body, vec![], Terminator::SwitchInt {
                    discr: copy_of(seen),
                    arms: vec![(0, first)],
                    otherwise: Some(compare),
                });
                fresh.block(
                    first,
                    vec![assign(seen, bool_const(true)), assign(k, Rvalue::Use(copy_of(i)))],
                    Terminator::Goto(step),
                );
                let (x, y, above) = (fresh.local((**elem).clone()), fresh.local((**elem).clone()), fresh.local(Ty::Bool));
                // max: the new one is kept unless the kept one is above it; min: only if the kept one is above it
                fresh.block(
                    compare,
                    vec![
                        assign(x, Rvalue::Use(Operand::Copy(at(k)))),
                        assign(y, Rvalue::Use(Operand::Copy(at(i)))),
                        assign(above, Rvalue::BinaryOp(ir::BinOp::Gt, copy_of(x), copy_of(y))),
                    ],
                    if max { branch(above, keep, step) } else { branch(above, step, keep) },
                );
                fresh.block(keep, vec![assign(k, Rvalue::Use(copy_of(i)))], Terminator::Goto(step));
                let kept = fresh.local((*r).clone());
                let some = done_block(
                    &mut fresh,
                    vec![
                        assign(kept, Rvalue::Ref(BorrowKind::Shared, at(k))),
                        set(variant(r, Some(copy_of(kept)))),
                    ],
                );
                let none = done_block(&mut fresh, vec![set(variant(r, None))]);
                let done = fresh.name();
                fresh.block(done, vec![], branch(seen, none, some));
                Some(done)
            }
            ("count", [_]) if o.elements.is_none() => {
                let n = fresh.local(Ty::Int(IntTy::Usize));
                entry.push(assign(n, Rvalue::Use(usize_const(0))));
                fresh.block(
                    body,
                    vec![assign(n, Rvalue::BinaryOp(ir::BinOp::Add, copy_of(n), usize_const(1)))],
                    Terminator::Goto(step),
                );
                Some(done_block(
                    &mut fresh,
                    vec![set(Rvalue::Use(Operand::Move(Place::local(n))))],
                ))
            }
            ("spec_nth" | "nth", [_, n]) if o.elements.is_none() => {
                let k = fresh.local(Ty::Int(IntTy::Usize));
                entry.push(assign(k, Rvalue::Use(n.clone())));
                let z = fresh.local(Ty::Bool);
                let found = done_block(&mut fresh, vec![set(variant(&it, Some(copy_of(i))))]);
                let down = fresh.name();
                fresh.block(
                    body,
                    vec![assign(z, Rvalue::BinaryOp(ir::BinOp::Eq, copy_of(k), usize_const(0)))],
                    branch(z, down, found),
                );
                fresh.block(
                    down,
                    vec![assign(k, Rvalue::BinaryOp(ir::BinOp::Sub, copy_of(k), usize_const(1)))],
                    Terminator::Goto(step),
                );
                Some(done_block(&mut fresh, vec![set(variant(&it, None))]))
            }
            _ => continue,
        };
        let Some(done) = looped else { continue };
        fresh.block(
            head,
            vec![assign(c, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(i), o.hi.clone()))],
            branch(c, done, body),
        );
        fresh.block(step, steps, Terminator::Goto(head));
        gone(f, &o.made);
        let b = &mut f.body.blocks[bi];
        b.statements.extend(entry);
        b.terminator = Terminator::Goto(head);
    }
    // the iterators' own lines, now nothing
    for b in f.body.blocks.iter_mut() {
        b.statements.retain(|s| *s != Statement::Nop);
    }
    f.body.decls.append(&mut fresh.decls);
    f.body.blocks.append(&mut fresh.blocks);
}

/// A slice of an array the function knows (`slices` made it a reference to `[T; N]`): where the array is, and the
/// indexes the slice covers.
fn slice_at(f: &Function, defs: &HashMap<Local, Vec<At>>, o: &Operand) -> Option<(Place, u64, u64)> {
    let l = o.local()?;
    let ty = f
        .params
        .iter()
        .find(|(p, _)| *p == l)
        .map(|(_, t)| t)
        .or_else(|| f.body.decls.iter().find(|d| d.local == l).map(|d| &d.ty))?;
    let (Ty::Ref { to, .. } | Ty::Ptr { to, .. }) = ty else {
        return None;
    };
    let Ty::Array(_, ArrayLen::Known(n)) = &**to else {
        return None;
    };
    let (a, from) = element_pointer(f, defs, o)?;
    Some((a, from, from + n))
}

/// A slice's `contains` and `fill` (rustc's `slice_contains` and `spec_fill` at opt-level 2), as loops over its
/// elements; and its `swap` and `reverse` where rustc calls them (opt-level 1; at 2 it inlines them into raw
/// slices and copies, which are refused).
pub fn slice_methods(f: &mut Function) {
    let mut fresh = Fresh::of(f);
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let impl_of_slice = matches!(func.segments.as_slice(), [Segment::Name { name, .. }, Segment::Impl(Ty::Slice(_)), _] if name == "slice");
        let contains = match (func.name(), args.as_slice()) {
            ("contains", [s, x]) if impl_of_slice => Some((s, x)),
            ("slice_contains", [x, s]) if func.qself.is_some() => Some((s, x)),
            _ => None,
        };
        let fill = match (func.name(), args.as_slice()) {
            ("fill", [s, v]) if impl_of_slice => Some((s, v)),
            ("spec_fill", [s, v]) if func.qself.is_some() => Some((s, v)),
            _ => None,
        };
        let defs = definitions(f);
        // `reverse`'s own swaps, as rustc inlines it: `revswap(a, b, n)` exchanges a's first n elements with b's
        // last n, last first
        let in_reverse = names(&func).is_some_and(|n| n.ends_with(&["reverse", "revswap"]))
            || matches!(func.segments.as_slice(), [Segment::Name { name: s, .. }, Segment::Impl(Ty::Slice(_)), Segment::Name { name: r, .. }, Segment::Name { name: w, .. }] if s == "slice" && r == "reverse" && w == "revswap");
        if let (true, [a, b, n]) = (in_reverse, args.as_slice()) {
            let (Some((pa, a0)), Some((pb, b0)), Some((n, _))) =
                (element_pointer(f, &defs, a), element_pointer(f, &defs, b), n.int())
            else {
                continue;
            };
            let Some(elem) = array_element(f, &pa) else { continue };
            let n = n as u64;
            let len = |p: &Place| match f
                .params
                .iter()
                .map(|(l, t)| (*l, t))
                .chain(f.body.decls.iter().map(|d| (d.local, &d.ty)))
                .find(|(l, _)| Some(*l) == p.base_local())
                .map(|(_, t)| t)
            {
                Some(Ty::Array(_, ArrayLen::Known(m))) => Some(*m),
                Some(Ty::Ref { to, .. }) => match &**to {
                    Ty::Array(_, ArrayLen::Known(m)) => Some(*m),
                    _ => None,
                },
                _ => None,
            };
            let (Some(la), Some(lb)) = (len(&pa), len(&pb)) else { continue };
            if a0 + n > la || b0 + n > lb || n > 8 {
                continue;
            }
            let at = |p: &Place, k: u64, m: u64| {
                let mut e = p.clone();
                e.projection.push(Projection::ConstantIndex { offset: k, min_length: m });
                e
            };
            let mut swaps = vec![];
            for k in 0..n {
                let (x, y) = (fresh.local(elem.clone()), fresh.local(elem.clone()));
                let (ea, eb) = (at(&pa, a0 + k, la), at(&pb, b0 + n - 1 - k, lb));
                swaps.extend([
                    assign(x, Rvalue::Use(Operand::Copy(ea.clone()))),
                    assign(y, Rvalue::Use(Operand::Copy(eb.clone()))),
                    Statement::Assign(ea, Rvalue::Use(copy_of(y))),
                    Statement::Assign(eb, Rvalue::Use(copy_of(x))),
                ]);
            }
            let b = &mut f.body.blocks[bi];
            b.statements.extend(swaps);
            b.terminator = Terminator::Goto(ret);
            continue;
        }
        if impl_of_slice && matches!(func.name(), "swap" | "reverse") {
            swap_or_reverse(f, &mut fresh, &defs, bi, func.name(), &args, ret);
            continue;
        }
        let Some((s, other)) = contains.or(fill) else { continue };
        let Some((a, from, to)) = slice_at(f, &defs, s) else {
            continue;
        };
        let i = fresh.local(Ty::Int(IntTy::Usize));
        let c = fresh.local(Ty::Bool);
        let (head, body, step, done) = (fresh.name(), fresh.name(), fresh.name(), fresh.name());
        let mut element = a.clone();
        element.projection.push(Projection::Index(i));
        let elem_ty = match f
            .body
            .decls
            .iter()
            .find(|d| Some(d.local) == a.base_local())
            .map(|d| &d.ty)
            .or_else(|| {
                f.params
                    .iter()
                    .find(|(p, _)| Some(*p) == a.base_local())
                    .map(|(_, t)| t)
            }) {
            Some(Ty::Array(e, _)) => (**e).clone(),
            Some(Ty::Ref { to, .. }) => match &**to {
                Ty::Array(e, _) => (**e).clone(),
                _ => continue,
            },
            _ => continue,
        };
        // the value to find or to fill with, read once, before the loop
        let v = fresh.local(elem_ty.clone());
        let read = match (contains.is_some(), other) {
            (true, x) => {
                let Some(x) = x.local() else { continue };
                Rvalue::Use(Operand::Copy(Place {
                    base: PlaceBase::Local(x),
                    projection: vec![Projection::Deref],
                }))
            }
            (false, v) => Rvalue::Use(v.clone()),
        };
        let mut entry = vec![assign(v, read), assign(i, Rvalue::Use(usize_const(from)))];
        if contains.is_some() {
            let e = fresh.local(elem_ty.clone());
            let eq = fresh.local(Ty::Bool);
            let yes = fresh.name();
            fresh.block(
                body,
                vec![
                    assign(e, Rvalue::Use(Operand::Copy(element))),
                    assign(eq, Rvalue::BinaryOp(ir::BinOp::Eq, copy_of(e), copy_of(v))),
                ],
                branch(eq, step, yes),
            );
            let set = |b: bool| Statement::Assign(dest.clone(), Rvalue::Use(Operand::Constant(Const::Bool(b))));
            fresh.block(yes, vec![set(true)], Terminator::Goto(ret));
            fresh.block(done, vec![set(false)], Terminator::Goto(ret));
        } else {
            fresh.block(
                body,
                vec![Statement::Assign(element, Rvalue::Use(copy_of(v)))],
                Terminator::Goto(step),
            );
            fresh.block(done, vec![], Terminator::Goto(ret));
        }
        fresh.block(
            head,
            vec![assign(c, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(i), usize_const(to)))],
            branch(c, done, body),
        );
        fresh.block(
            step,
            vec![assign(i, Rvalue::BinaryOp(ir::BinOp::Add, copy_of(i), usize_const(1)))],
            Terminator::Goto(head),
        );
        let b = &mut f.body.blocks[bi];
        b.statements.append(&mut entry);
        b.terminator = Terminator::Goto(head);
    }
    f.body.decls.append(&mut fresh.decls);
    f.body.blocks.append(&mut fresh.blocks);
}

/// `a == b` (or `!=`) as one element after another, each an `==` and a test, ending at the first that differs.
/// The blocks from `first`; each way out sets `dest` and goes on to `then`.
fn equal_elements(
    fresh: &mut Fresh,
    pairs: &[(Place, Place)],
    elem: &Ty,
    dest: Place,
    negate: bool,
    then: BasicBlock,
) -> BasicBlock {
    let (yes, no) = (fresh.name(), fresh.name());
    let set = |b: bool| Statement::Assign(dest.clone(), Rvalue::Use(Operand::Constant(Const::Bool(b != negate))));
    fresh.block(yes, vec![set(true)], Terminator::Goto(then));
    fresh.block(no, vec![set(false)], Terminator::Goto(then));
    let mut next = yes;
    for (a, b) in pairs.iter().rev() {
        let (x, y, e) = (
            fresh.local(elem.clone()),
            fresh.local(elem.clone()),
            fresh.local(Ty::Bool),
        );
        let name = fresh.name();
        fresh.block(
            name,
            vec![
                assign(x, Rvalue::Use(Operand::Copy(a.clone()))),
                assign(y, Rvalue::Use(Operand::Copy(b.clone()))),
                assign(e, Rvalue::BinaryOp(ir::BinOp::Eq, copy_of(x), copy_of(y))),
            ],
            branch(e, no, next),
        );
        next = name;
    }
    next
}

/// The elements of two arrays of n, pair by pair (each array where `element_pointer` found it, from its start).
fn element_pairs(a: &Place, b: &Place, n: u64) -> Vec<(Place, Place)> {
    let at = |p: &Place, k: u64| {
        let mut e = p.clone();
        e.projection.push(Projection::ConstantIndex {
            offset: k,
            min_length: n,
        });
        e
    };
    (0..n).map(|k| (at(a, k), at(b, k))).collect()
}

/// `==` and `!=` of two arrays of a cell's type: rustc's call of `<[T; N] as PartialEq>::eq` (opt-level 1), or its
/// compare of both arrays' bytes as one integer (`copy a as u64 (Transmute)`, at 2), as the elements compared one
/// after another. At opt-level 2 only for arrays assigned once and never written, as the copy in the integer is
/// taken where it is.
pub fn array_eq(f: &mut Function) {
    let mut fresh = Fresh::of(f);
    let element = |t: Option<&Ty>| match t {
        Some(Ty::Array(e, ArrayLen::Known(n))) if matches!(**e, Ty::Int(IntTy::I16 | IntTy::U8) | Ty::Bool) => {
            Some(((**e).clone(), *n))
        }
        _ => None,
    };
    let types = |f: &Function| {
        let mut t: HashMap<Local, Ty> = f.params.iter().cloned().collect();
        t.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
        t
    };
    // rustc's call, at opt-level 1
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let Some(q) = &func.qself else { continue };
        let partial_eq = q.as_trait.as_ref().is_some_and(|t| names(t) == Some(vec!["PartialEq"]));
        let negate = match func.name() {
            "eq" => false,
            "ne" => true,
            _ => continue,
        };
        let (Some((elem, n)), true, [x, y]) = (element(Some(&q.ty)), partial_eq, args.as_slice()) else {
            continue;
        };
        let defs = definitions(f);
        let (Some((a, 0)), Some((b, 0))) = (element_pointer(f, &defs, x), element_pointer(f, &defs, y)) else {
            continue;
        };
        let first = equal_elements(&mut fresh, &element_pairs(&a, &b, n), &elem, dest, negate, ret);
        f.body.blocks[bi].terminator = Terminator::Goto(first);
    }
    // the integer compare, at opt-level 2
    loop {
        let t = types(f);
        let defs = definitions(f);
        let bytes = |o: &Operand| -> Option<(Local, Local, (usize, usize))> {
            let x = o.local()?;
            if !matches!(
                t.get(&x),
                Some(Ty::Int(IntTy::U16 | IntTy::U32 | IntTy::U64 | IntTy::U128))
            ) || ir::occurrences(&f.body.blocks, x) != 2
            {
                return None;
            }
            let [At::Line(bi, si)] = defs.get(&x)?.as_slice() else {
                return None;
            };
            let Statement::Assign(_, Rvalue::Cast(a, _, CastKind::Transmute)) = &f.body.blocks[*bi].statements[*si]
            else {
                return None;
            };
            let a = a.local()?;
            let [d] = defs.get(&a)?.as_slice() else { return None };
            (element(t.get(&a)).is_some() && writes(f, a, *d) == 0).then_some((x, a, (*bi, *si)))
        };
        let found = f.body.blocks.iter().enumerate().find_map(|(bi, b)| {
            b.statements.iter().enumerate().find_map(|(si, s)| match s {
                Statement::Assign(d, Rvalue::BinaryOp(op @ (ir::BinOp::Eq | ir::BinOp::Ne), x, y)) => {
                    let (x, y) = (bytes(x)?, bytes(y)?);
                    (t.get(&x.1) == t.get(&y.1)).then(|| (bi, si, d.clone(), *op == ir::BinOp::Ne, x, y))
                }
                _ => None,
            })
        });
        let Some((bi, si, dest, negate, (_, a, ca), (_, b, cb))) = found else {
            break;
        };
        let (elem, n) = element(t.get(&a)).expect("an array, as found");
        for (ci, cj) in [ca, cb] {
            f.body.blocks[ci].statements[cj] = Statement::Nop;
        }
        // the block cut at the compare: what follows it, on to the block's own end
        let then = fresh.name();
        let blk = &mut f.body.blocks[bi];
        let rest = blk.statements.split_off(si + 1);
        blk.statements.pop();
        let end = std::mem::replace(&mut blk.terminator, Terminator::Unreachable);
        fresh.block(then, rest, end);
        let first = equal_elements(
            &mut fresh,
            &element_pairs(&Place::local(a), &Place::local(b), n),
            &elem,
            dest,
            negate,
            then,
        );
        f.body.blocks[bi].terminator = Terminator::Goto(first);
        // (the new blocks join the body now: the next compare may be in one of them)
        f.body.blocks.append(&mut fresh.blocks);
    }
    for b in f.body.blocks.iter_mut() {
        b.statements.retain(|s| *s != Statement::Nop);
    }
    f.body.decls.append(&mut fresh.decls);
    f.body.blocks.append(&mut fresh.blocks);
    let blocks = &f.body.blocks;
    f.body.decls.retain(|d| {
        !matches!(d.ty, Ty::Int(IntTy::U16 | IntTy::U32 | IntTy::U64 | IntTy::U128))
            || ir::occurrences(blocks, d.local) > 0
    });
}

/// `core::ops::Range<T>`, or the struct the for-loop pass makes of one (`Range_i16`), for T a card's type: T.
pub(super) fn card_range(t: &Ty) -> Option<IntTy> {
    let Ty::Path(p) = t else { return None };
    let card = |t: &Ty| match t {
        Ty::Int(t @ (IntTy::I16 | IntTy::U8 | IntTy::Usize)) => Some(*t),
        _ => None,
    };
    if p.qself.is_none() && names(p)? == ["core", "ops", "Range"] {
        return match last_args(p).as_slice() {
            [GenericArg::Ty(t)] => card(t),
            _ => None,
        };
    }
    match p.is_plain().then(|| p.name())? {
        "Range_i16" => Some(IntTy::I16),
        "Range_u8" => Some(IntTy::U8),
        "Range_usize" => Some(IntTy::Usize),
        _ => None,
    }
}

/// `core::iter::StepBy<R>` for R a range of a card's type: the card's type.
fn step_by_ty(t: &Ty) -> Option<IntTy> {
    let Ty::Path(p) = t else { return None };
    let [GenericArg::Ty(r)] = last_args(p).as_slice() else {
        return None;
    };
    // (a method's path names it `StepBy<..>`, a local's type `core::iter::StepBy<..>`)
    let n = names(p)?;
    (p.qself.is_none() && (n == ["core", "iter", "StepBy"] || n == ["StepBy"])).then(|| card_range(r))?
}

fn range_struct(t: IntTy) -> Ty {
    Ty::Path(path(&[&format!("Range_{t}")], vec![]))
}

fn step_by_struct(t: IntTy) -> Ty {
    Ty::Path(path(&[&format!("StepBy_{t}")], vec![]))
}

/// `(a..b).step_by(k)`: core's `StepBy` of a range, a plain struct of the range, k - 1 and whether the first item
/// is still to come, as core has it, which the struct pass takes apart. `StepBy::new` checks k isn't 0 and builds
/// it; what rustc inlines of its `next()` is left as it is, but for the `nth()` of the range it calls, which is
/// worked out here: the start moved on by n if that is short of the end, as an `i16` past a card can't hold the
/// sum, the test is split by the start's sign (from 0 or more, the distance to the end fits a card; from below 0,
/// the sum does).
pub fn step_by(f: &mut Function) {
    let steps: Vec<(Local, IntTy)> = f
        .body
        .decls
        .iter()
        .filter_map(|d| step_by_ty(&d.ty).map(|t| (d.local, t)))
        .collect();
    if steps.is_empty() {
        return;
    }
    let mut fresh = Fresh::of(f);
    let is_step = |l: Local| steps.iter().find(|(s, _)| *s == l).map(|(_, t)| *t);
    let mut made_any = false;
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let Some(d) = dest.as_local() else { continue };
        let Some(t) = is_step(d) else { continue };
        // `StepBy::<R>::new(r, k)` (opt-level 2), or `<R as Iterator>::step_by(r, k)` (1)
        let new = func.name() == "new" && names(&func).is_some_and(|n| n.len() >= 2 && n[n.len() - 2] == "StepBy");
        let method = func.name() == "step_by" && func.qself.as_ref().is_some_and(|q| card_range(&q.ty) == Some(t));
        let ([r, k], true) = (args.as_slice(), new || method) else {
            continue;
        };
        let c = fresh.local(Ty::Bool);
        let m = fresh.local(Ty::Int(IntTy::Usize));
        let build = fresh.name();
        let take = |o: &Operand| match o {
            Operand::Move(p) => Operand::Copy(p.clone()),
            o => o.clone(),
        };
        fresh.block(
            build,
            vec![
                assign(m, Rvalue::BinaryOp(ir::BinOp::Sub, take(k), usize_const(1))),
                Statement::Assign(
                    dest.clone(),
                    Rvalue::Struct(
                        path(&[&format!("StepBy_{t}")], vec![]),
                        vec![
                            ("iter".into(), r.clone()),
                            ("step_minus_one".into(), Operand::Move(Place::local(m))),
                            ("first_take".into(), Operand::Constant(Const::Bool(true))),
                        ],
                    ),
                ),
            ],
            Terminator::Goto(ret),
        );
        let b = &mut f.body.blocks[bi];
        b.statements
            .push(assign(c, Rvalue::BinaryOp(ir::BinOp::Ne, take(k), usize_const(0))));
        b.terminator = Terminator::Assert {
            cond: Operand::Move(Place::local(c)),
            expected: true,
            msg: "assertion failed: step != 0".into(),
            msg_args: vec![],
            target: build,
            unwind: Unwind::Continue,
        };
        made_any = true;
    }
    if !made_any {
        return;
    }
    // where rustc calls them (opt-level 1): the StepBy's into_iter(), itself, and its next(), core's: the range's
    // nth() of 0 the first time, of k - 1 after
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let Some(t) = func.qself.as_ref().and_then(|q| step_by_ty(&q.ty)) else { continue };
        let [Operand::Move(a)] = args.as_slice() else { continue };
        let Some(a) = a.as_local() else { continue };
        if func.name() == "into_iter" && is_step(a).is_some() {
            let b = &mut f.body.blocks[bi];
            b.statements.push(Statement::Assign(dest.clone(), Rvalue::Use(Operand::Move(Place::local(a)))));
            b.terminator = Terminator::Goto(ret);
            continue;
        }
        if func.name() != "next" || ir::occurrences(&f.body.blocks, a) != 2 {
            continue;
        }
        let Some(at) = f.body.blocks[bi].statements.iter().position(|s| {
            matches!(s, Statement::Assign(r, Rvalue::Ref(BorrowKind::Mut, p)) if r.as_local() == Some(a) && p.projection.is_empty())
        }) else {
            continue;
        };
        let Statement::Assign(_, Rvalue::Ref(_, it)) = f.body.blocks[bi].statements.remove(at) else { unreachable!() };
        let Some(s) = it.base_local().filter(|s| is_step(*s).is_some()) else { continue };
        let field = |k: u32, ty: Ty| Place::field(s, k, ty);
        let (ft, n, r) = (
            fresh.local(Ty::Bool),
            fresh.local(Ty::Int(IntTy::Usize)),
            fresh.local(Ty::Ref { lifetime: None, mutable: true, to: Box::new(range_struct(t)) }),
        );
        let (first, later, call_at) = (fresh.name(), fresh.name(), fresh.name());
        let nth = Path {
            qself: Some(Box::new(QSelf {
                ty: range_struct(t),
                as_trait: Some(path(&["iter", "range", "RangeIteratorImpl"], vec![])),
            })),
            segments: vec![Segment::Name { name: "spec_nth".into(), args: None }],
        };
        fresh.block(first, vec![assign(n, Rvalue::Use(usize_const(0)))], Terminator::Goto(call_at));
        fresh.block(
            later,
            vec![assign(n, Rvalue::Use(Operand::Copy(field(1, Ty::Int(IntTy::Usize)))))],
            Terminator::Goto(call_at),
        );
        fresh.block(
            call_at,
            vec![assign(r, Rvalue::Ref(BorrowKind::Mut, field(0, range_struct(t))))],
            Terminator::Call {
                dest,
                func: nth,
                args: vec![Operand::Move(Place::local(r)), Operand::Move(Place::local(n))],
                target: Some(ret),
                unwind: Unwind::Continue,
            },
        );
        let b = &mut f.body.blocks[bi];
        b.statements.extend([
            assign(ft, Rvalue::Use(Operand::Copy(field(2, Ty::Bool)))),
            Statement::Assign(field(2, Ty::Bool), Rvalue::Use(Operand::Constant(Const::Bool(false)))),
        ]);
        b.terminator = branch(ft, later, first);
    }
    // (the blocks just made hold nth() calls of their own)
    f.body.blocks.append(&mut fresh.blocks);
    // the range's nth(), on the StepBy's range
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let ("spec_nth", [Operand::Move(r), n]) = (func.name(), args.as_slice()) else {
            continue;
        };
        let Some(r) = r.as_local() else { continue };
        let Some(range) = f.body.blocks[bi].statements.iter().find_map(|s| match s {
            Statement::Assign(d, Rvalue::Ref(BorrowKind::Mut, p)) if d.as_local() == Some(r) => Some(p.clone()),
            _ => None,
        }) else {
            continue;
        };
        let (Some(s), [Projection::Field(0, _)]) = (range.base_local(), range.projection.as_slice()) else {
            continue;
        };
        let Some(t) = is_step(s) else { continue };
        if ir::occurrences(&f.body.blocks, r) != 2 {
            continue;
        }
        let mut range = range.clone();
        range.projection = vec![Projection::Field(0, range_struct(t))];
        let field = |k: u32| {
            let mut p = range.clone();
            p.projection.push(Projection::Field(k, Ty::Int(t)));
            p
        };
        let card = Ty::Int(t);
        let (st, en, nn, plus) = (
            fresh.local(card.clone()),
            fresh.local(card.clone()),
            fresh.local(card.clone()),
            fresh.local(card.clone()),
        );
        let (c0, c1, c2) = (fresh.local(Ty::Bool), fresh.local(Ty::Bool), fresh.local(Ty::Bool));
        let (none, some, pos, neg, pos_some) = (fresh.name(), fresh.name(), fresh.name(), fresh.name(), fresh.name());
        let one = Operand::Constant(Const::Int { value: 1, ty: t });
        let zero = Operand::Constant(Const::Int { value: 0, ty: t });
        fresh.block(
            none,
            vec![
                Statement::Assign(field(0), Rvalue::Use(copy_of(en))),
                Statement::Assign(dest.clone(), variant(&card, None)),
            ],
            Terminator::Goto(ret),
        );
        fresh.block(
            some,
            vec![
                Statement::Assign(field(0), Rvalue::BinaryOp(ir::BinOp::Add, copy_of(plus), one)),
                Statement::Assign(dest.clone(), variant(&card, Some(copy_of(plus)))),
            ],
            Terminator::Goto(ret),
        );
        // from 0 or more: n against the distance to the end
        let gap = fresh.local(card.clone());
        fresh.block(
            pos,
            vec![
                assign(gap, Rvalue::BinaryOp(ir::BinOp::Sub, copy_of(en), copy_of(st))),
                assign(c1, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(nn), copy_of(gap))),
            ],
            branch(c1, none, pos_some),
        );
        fresh.block(
            pos_some,
            vec![assign(plus, Rvalue::BinaryOp(ir::BinOp::Add, copy_of(st), copy_of(nn)))],
            Terminator::Goto(some),
        );
        // from below 0: the sum, against the end
        fresh.block(
            neg,
            vec![
                assign(plus, Rvalue::BinaryOp(ir::BinOp::Add, copy_of(st), copy_of(nn))),
                assign(c2, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(plus), copy_of(en))),
            ],
            branch(c2, none, some),
        );
        let signed = fresh.name();
        let below = fresh.local(Ty::Bool);
        fresh.block(
            signed,
            vec![assign(below, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(st), zero))],
            branch(below, pos, neg),
        );
        let n_card = if t == IntTy::Usize {
            Rvalue::Use(n.clone())
        } else {
            Rvalue::Cast(n.clone(), card.clone(), CastKind::IntToInt)
        };
        let b = &mut f.body.blocks[bi];
        b.statements.retain(|s| s.def() != Some(r));
        b.statements.extend([
            assign(st, Rvalue::Use(Operand::Copy(field(0)))),
            assign(en, Rvalue::Use(Operand::Copy(field(1)))),
            assign(nn, n_card),
            assign(c0, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(st), copy_of(en))),
        ]);
        b.terminator = branch(c0, none, if t == IntTy::I16 { signed } else { pos });
    }
    // the types: StepBy and the range inside it as the plain structs, wherever a StepBy's fields are named
    for d in f.body.decls.iter_mut() {
        if let Some(t) = step_by_ty(&d.ty) {
            d.ty = step_by_struct(t);
        }
    }
    for b in f.body.blocks.iter_mut() {
        let mut fix = |p: &mut Place| {
            if let (Some(t), Some(Projection::Field(0, ft))) =
                (p.base_local().and_then(is_step), p.projection.first_mut())
            {
                if card_range(ft).is_some() {
                    *ft = range_struct(t);
                }
            }
        };
        for s in b.statements.iter_mut() {
            s.places_mut(&mut fix);
        }
        b.terminator.places_mut(&mut fix);
    }
    f.body.decls.append(&mut fresh.decls);
    f.body.blocks.append(&mut fresh.blocks);
}

/// The element at a run-time index of a slice from the start of its array.
fn at_index(a: &Place, i: Local) -> Place {
    let mut e = a.clone();
    e.projection.push(Projection::Index(i));
    e
}

/// `s.swap(i, j)`: both indexes checked against the length, as Rust does, then the two elements exchanged; and
/// `s.reverse()`: the elements exchanged from both ends inwards. For a slice from its array's start.
fn swap_or_reverse(
    f: &mut Function,
    fresh: &mut Fresh,
    defs: &HashMap<Local, Vec<At>>,
    bi: usize,
    method: &str,
    args: &[Operand],
    ret: BasicBlock,
) {
    let Some(s) = args.first() else { return };
    let Some((a, 0, n)) = slice_at(f, defs, s) else { return };
    let elem = {
        let base = a.base_local();
        let ty = f.params.iter().find(|(p, _)| Some(*p) == base).map(|(_, t)| t).or_else(|| {
            f.body.decls.iter().find(|d| Some(d.local) == base).map(|d| &d.ty)
        });
        match ty {
            Some(Ty::Array(e, _)) => (**e).clone(),
            Some(Ty::Ref { to, .. }) => match &**to {
                Ty::Array(e, _) => (**e).clone(),
                _ => return,
            },
            _ => return,
        }
    };
    let (x, y) = (fresh.local(elem.clone()), fresh.local(elem));
    // the two elements, i and j, exchanged
    let exchange = |i: Local, j: Local| {
        vec![
            assign(x, Rvalue::Use(Operand::Copy(at_index(&a, i)))),
            assign(y, Rvalue::Use(Operand::Copy(at_index(&a, j)))),
            Statement::Assign(at_index(&a, i), Rvalue::Use(copy_of(y))),
            Statement::Assign(at_index(&a, j), Rvalue::Use(copy_of(x))),
        ]
    };
    let (i, j) = (fresh.local(Ty::Int(IntTy::Usize)), fresh.local(Ty::Int(IntTy::Usize)));
    match (method, args) {
        ("swap", [_, ia, ja]) => {
            let checked = |fresh: &mut Fresh, k: Local, then: BasicBlock| {
                let c = fresh.local(Ty::Bool);
                let name = fresh.name();
                fresh.block(
                    name,
                    vec![assign(c, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(k), usize_const(n)))],
                    Terminator::Assert {
                        cond: Operand::Move(Place::local(c)),
                        expected: true,
                        msg: "index out of bounds: the length is {} but the index is {}".into(),
                        msg_args: vec![usize_const(n), copy_of(k)],
                        target: then,
                        unwind: Unwind::Continue,
                    },
                );
                name
            };
            let swap = fresh.name();
            fresh.block(swap, exchange(i, j), Terminator::Goto(ret));
            let second = checked(fresh, j, swap);
            let first = checked(fresh, i, second);
            let b = &mut f.body.blocks[bi];
            b.statements.push(assign(i, Rvalue::Use(ia.clone())));
            b.statements.push(assign(j, Rvalue::Use(ja.clone())));
            b.terminator = Terminator::Goto(first);
        }
        ("reverse", [_]) if n >= 2 => {
            // i up from the start and j down from the end, while i < j
            let (head, body) = (fresh.name(), fresh.name());
            let c = fresh.local(Ty::Bool);
            fresh.block(
                head,
                vec![assign(c, Rvalue::BinaryOp(ir::BinOp::Lt, copy_of(i), copy_of(j)))],
                branch(c, ret, body),
            );
            let mut step = exchange(i, j);
            step.push(assign(i, Rvalue::BinaryOp(ir::BinOp::Add, copy_of(i), usize_const(1))));
            step.push(assign(j, Rvalue::BinaryOp(ir::BinOp::Sub, copy_of(j), usize_const(1))));
            fresh.block(body, step, Terminator::Goto(head));
            let b = &mut f.body.blocks[bi];
            b.statements.push(assign(i, Rvalue::Use(usize_const(0))));
            b.statements.push(assign(j, Rvalue::Use(usize_const(n - 1))));
            b.terminator = Terminator::Goto(head);
        }
        ("reverse", [_]) => f.body.blocks[bi].terminator = Terminator::Goto(ret),
        _ => {}
    }
}

/// The element type of the array a place (an array local, or one behind a pointer) holds.
fn array_element(f: &Function, a: &Place) -> Option<Ty> {
    let base = a.base_local();
    let ty = f.params.iter().find(|(p, _)| Some(*p) == base).map(|(_, t)| t).or_else(|| {
        f.body.decls.iter().find(|d| Some(d.local) == base).map(|d| &d.ty)
    })?;
    match ty {
        Ty::Array(e, _) => Some((**e).clone()),
        Ty::Ref { to, .. } | Ty::Ptr { to, .. } => match &**to {
            Ty::Array(e, _) => Some((**e).clone()),
            _ => None,
        },
        _ => None,
    }
}
