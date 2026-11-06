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
fn path(names: &[&str], args: Vec<Ty>) -> Path {
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
fn option(t: Ty) -> Ty {
    Ty::Path(path(&["core", "option", "Option"], vec![t]))
}

fn usize_const(v: u64) -> Operand {
    Operand::Constant(Const::Int {
        value: v as i128,
        ty: IntTy::Usize,
    })
}

/// The names of a path's segments, if all are plain names (type arguments aside).
fn names(p: &Path) -> Option<Vec<&str>> {
    p.segments
        .iter()
        .map(|s| match s {
            Segment::Name { name, .. } => Some(name.as_str()),
            Segment::Impl(_) => None,
        })
        .collect()
}

/// The last segment's type arguments.
fn last_args(p: &Path) -> Vec<&GenericArg> {
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
        [GenericArg::Ty(e @ (Ty::Int(IntTy::I16 | IntTy::U8) | Ty::Bool)), GenericArg::Const(n)] => {
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
enum At {
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
fn definitions(f: &Function) -> HashMap<Local, Vec<At>> {
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
    for b in f.body.blocks.iter_mut() {
        if let Terminator::Call { unwind, .. } = &mut b.terminator {
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
fn element_pointer(f: &Function, defs: &HashMap<Local, Vec<At>>, o: &Operand) -> Option<(Place, u64)> {
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
        Rvalue::BinaryOp(ir::BinOp::Add, o, Operand::Constant(Const::Int { value, .. })) => {
            let (a, k) = element_pointer(f, defs, o)?;
            Some((a, k + u64::try_from(*value).ok()?))
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
