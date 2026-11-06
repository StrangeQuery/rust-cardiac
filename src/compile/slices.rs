//! Slices of arrays, and the pointers core's slice iterators are made of. rustc borrows an array as a slice
//! (`&[i16]`) for `for x in &t`, `t.iter()` and the array methods, and at opt-level 2 it inlines `slice::Iter` into
//! pointer arithmetic: a pointer to the first element and one past the last, stepped by `Offset` and compared.
//! On CARDIAC a pointer is a cell address and every element one cell (front.rs makes `Offset` an `Add`), so all of
//! that is already code the compiler writes, once the types are ones it knows:
//!   a slice made from an array of known length N (every definition of it, through copies, casts and reborrows) is
//!     a reference to `[T; N]`, one cell: its address; its length (`PtrMetadata`) is the constant N
//!   `NonNull<T>` is a `*mut T`, and `MaybeUninit<T>` a T
//!   `&raw const (*p)`, `&(*p)` and `&mut (*p)` of a pointer p are p
//!   `slice::Iter<'_, T>` and `slice::IterMut<'_, T>` are a plain struct of their two pointers, `ptr` and
//!     `end_or_len` (its `PhantomData` field goes), which the struct pass takes apart
//!   `copy t[1..4]`, the elements a rest pattern binds (`[x, rest @ ..]`), is the array `[t[1], t[2], t[3]]`
//! A slice whose length isn't known here (a parameter, say) is left as it is, and refused.
use std::collections::{HashMap, HashSet};

use super::ir::visit_mut::{walk_rvalue, walk_ty, VisitMut};
use super::ir::{
    self, ArrayLen, BasicBlock, BinOp, Block, BorrowKind, CastKind, Const, GenericArg, IntTy, Local, Operand, Path,
    Place, PlaceBase, Projection, Rvalue, Segment, Statement, Terminator, Ty,
};
use super::structs::Function;

/// `core::ptr::NonNull<T>`'s T.
fn non_null(p: &Path) -> Option<&Ty> {
    let [Segment::Name { name: a, args: None }, Segment::Name { name: b, args: None }, Segment::Name { name, args: Some(g) }] =
        p.segments.as_slice()
    else {
        return None;
    };
    if p.qself.is_some() || a != "core" || b != "ptr" || name != "NonNull" {
        return None;
    }
    match g.args.as_slice() {
        [GenericArg::Ty(t)] => Some(t),
        _ => None,
    }
}

/// `core::mem::MaybeUninit<T>`'s T, for T a card's type: a cell holds some value, and the value is the T (an
/// array's `IntoIter` holds its elements so, and `swap` its element in passing).
fn maybe_uninit(p: &Path) -> Option<&Ty> {
    let [Segment::Name { name: a, args: None }, Segment::Name { name: b, args: None }, Segment::Name { name, args: Some(g) }] =
        p.segments.as_slice()
    else {
        return None;
    };
    if p.qself.is_some() || a != "core" || b != "mem" || name != "MaybeUninit" {
        return None;
    }
    match g.args.as_slice() {
        [GenericArg::Ty(t @ (Ty::Int(IntTy::I16 | IntTy::U8) | Ty::Bool))] => Some(t),
        _ => None,
    }
}

/// `core::slice::Iter<'_, T>` or `core::slice::IterMut<'_, T>` for an element type T a cell holds: the struct it
/// becomes, by name.
fn slice_iter(p: &Path) -> Option<String> {
    let [Segment::Name { name: a, args: None }, Segment::Name { name: b, args: None }, Segment::Name { name, args: Some(g) }] =
        p.segments.as_slice()
    else {
        return None;
    };
    if p.qself.is_some() || a != "core" || b != "slice" || (name != "Iter" && name != "IterMut") {
        return None;
    }
    let t = g.args.iter().find_map(|a| match a {
        GenericArg::Ty(t @ (Ty::Int(IntTy::I16 | IntTy::U8) | Ty::Bool)) => Some(t),
        _ => None,
    })?;
    Some(format!("slice_{name}_{t}"))
}

fn plain(name: String) -> Path {
    Path {
        qself: None,
        segments: vec![Segment::Name { name, args: None }],
    }
}

/// NonNull as a raw pointer, and the slice iterators as plain structs, wherever a type is written.
struct Pointers;

impl VisitMut for Pointers {
    fn visit_ty(&mut self, t: &mut Ty) {
        walk_ty(self, t);
        let Ty::Path(p) = t else { return };
        if let Some(to) = non_null(p) {
            *t = Ty::Ptr {
                mutable: true,
                to: Box::new(to.clone()),
            };
        } else if let Some(name) = slice_iter(p) {
            *t = Ty::Path(plain(name));
        } else if let Some(v) = maybe_uninit(p) {
            *t = v.clone();
        }
    }

    fn visit_rvalue(&mut self, r: &mut Rvalue) {
        if let Rvalue::Struct(p, fields) = r {
            if let Some(name) = slice_iter(p) {
                *p = plain(name);
                fields.retain(|(n, _)| n != "_marker");
            }
        }
        walk_rvalue(self, r);
    }
}

/// A pointer to a slice, `&[T]`, `&mut [T]`, `*const [T]` or `*mut [T]`: where its type points.
fn slice_pointee(t: &Ty) -> Option<&Ty> {
    match t {
        Ty::Ref { to, .. } | Ty::Ptr { to, .. } => match &**to {
            Ty::Slice(_) => Some(to),
            _ => None,
        },
        _ => None,
    }
}

/// A pointer to an array of known length: the length.
fn array_length(t: &Ty) -> Option<u64> {
    match t {
        Ty::Ref { to, .. } | Ty::Ptr { to, .. } => match &**to {
            Ty::Array(_, ArrayLen::Known(n)) => Some(*n),
            _ => None,
        },
        _ => None,
    }
}

/// `*l` of a local l, and nothing more.
fn deref_of(p: &Place) -> Option<Local> {
    match p.projection.as_slice() {
        [Projection::Deref] => p.base_local(),
        _ => None,
    }
}

/// The length a definition of a slice pointer gives it, from its source's: `copy a as &[T] (Unsize)` of an array
/// pointer, or a copy, a cast or a reborrow of another slice pointer whose length is known.
fn defined_length(r: &Rvalue, types: &HashMap<Local, Ty>, known: &HashMap<Local, u64>) -> Option<u64> {
    let local = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    match r {
        Rvalue::Cast(o, _, CastKind::Unsize(_)) => array_length(types.get(&local(o)?)?),
        // a slice's pointer made of its parts, its length in the type (`ir::Rvalue::slice_from_parts`)
        Rvalue::Cast(_, t @ Ty::Ptr { .. }, CastKind::PtrToPtr) if array_length(t).is_some() => array_length(t),
        Rvalue::Cast(o, _, CastKind::Transmute | CastKind::PtrToPtr) | Rvalue::Use(o) => known.get(&local(o)?).copied(),
        Rvalue::Ref(_, p) => known.get(&deref_of(p)?).copied(),
        _ => None,
    }
}

/// The slices, pointers and slice iterators of a function in the terms above.
pub fn known_lengths(f: &mut Function) {
    Pointers.visit_body(&mut f.body);
    for (_, t) in f.params.iter_mut() {
        Pointers.visit_ty(t);
    }
    Pointers.visit_ty(&mut f.ret);
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    // each slice pointer's definitions: none from a parameter, or from anything but a whole assignment
    let slices: Vec<Local> = types
        .iter()
        .filter(|(l, t)| slice_pointee(t).is_some() && !f.params.iter().any(|(p, _)| p == *l))
        .map(|(l, _)| *l)
        .collect();
    let mut defs: HashMap<Local, Vec<Option<&Rvalue>>> = HashMap::new();
    for b in &f.body.blocks {
        for s in &b.statements {
            let Statement::Assign(d, r) = s else { continue };
            let Some(l) = d.base_local().filter(|l| slices.contains(l)) else {
                continue;
            };
            defs.entry(l).or_default().push(d.projection.is_empty().then_some(r));
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            if let Some(l) = dest.base_local().filter(|l| slices.contains(l)) {
                defs.entry(l).or_default().push(None);
            }
        }
    }
    // a length for each whose definitions all give the same one, in as many rounds as it takes
    let mut known: HashMap<Local, u64> = HashMap::new();
    loop {
        let mut more = false;
        for (l, ds) in &defs {
            if known.contains_key(l) {
                continue;
            }
            let lengths: Option<Vec<u64>> = ds
                .iter()
                .map(|d| d.and_then(|r| defined_length(r, &types, &known)))
                .collect();
            if let Some(n) = lengths
                .as_deref()
                .and_then(|ns| ns.first().filter(|n| ns.iter().all(|m| m == *n)))
            {
                known.insert(*l, *n);
                more = true;
            }
        }
        if !more {
            break;
        }
    }
    let array = |t: &mut Ty, n: u64| {
        if let Ty::Ref { to, .. } | Ty::Ptr { to, .. } = t {
            if let Ty::Slice(e) = &**to {
                **to = Ty::Array(e.clone(), ArrayLen::Known(n));
            }
        }
    };
    for d in f.body.decls.iter_mut() {
        if let Some(n) = known.get(&d.local) {
            array(&mut d.ty, *n);
            types.insert(d.local, d.ty.clone());
        }
    }
    // `core::intrinsics::copy::<T>(src, dst, 1)`, as a slice's swap moves an element: the copy itself
    for b in f.body.blocks.iter_mut() {
        let Terminator::Call {
            func,
            args,
            target: Some(t),
            ..
        } = &b.terminator
        else {
            continue;
        };
        let [src, dst, count] = args.as_slice() else { continue };
        let intrinsic = func.qself.is_none() && func.segments.len() == 3 && {
            let n: Vec<&str> = func
                .segments
                .iter()
                .filter_map(|s| match s {
                    Segment::Name { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            n == ["core", "intrinsics", "copy"]
        };
        if let (true, Some(s)) = (intrinsic, Statement::copy_one(src.clone(), dst.clone(), count.clone())) {
            let t = *t;
            b.statements.push(s);
            b.terminator = Terminator::Goto(t);
        }
    }
    let pointer = |l: Local| matches!(types.get(&l), Some(Ty::Ref { .. } | Ty::Ptr { .. }));
    let iters = slice_iter_fields(f, &types);
    let field_of = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local().and_then(|l| iters.get(&l)).copied(),
        _ => None,
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            let Statement::Assign(d, r) = s else { continue };
            let dest = d.as_local();
            if let (Some(n), Rvalue::Cast(_, t, _)) = (dest.and_then(|l| known.get(&l)), &mut *r) {
                array(t, *n);
            }
            // a pointer made another kind of pointer (or an array's a slice's) is the same address
            if let Rvalue::Cast(o, _, CastKind::Unsize(_) | CastKind::Transmute | CastKind::PtrToPtr) = &*r {
                let from = match o {
                    Operand::Copy(p) | Operand::Move(p) => p.as_local(),
                    _ => None,
                };
                if from.is_some_and(pointer) && dest.is_some_and(pointer) {
                    *r = Rvalue::Use(o.clone());
                }
            }
            if let Rvalue::Ref(BorrowKind::Shared | BorrowKind::Mut | BorrowKind::RawConst | BorrowKind::RawMut, p) =
                &*r
            {
                if let Some(q) = deref_of(p).filter(|q| pointer(*q)) {
                    *r = Rvalue::Use(Operand::Copy(Place::local(q)));
                }
            }
            // an iterator's own test of its pointer against its end: the pointer never passes the end
            if let Rvalue::BinaryOp(op @ (BinOp::Eq | BinOp::Ne), x, y) = r {
                if let (Some((a, 0)), Some((b, 1))) = (field_of(x), field_of(y)) {
                    if a == b {
                        *op = if *op == BinOp::Eq { BinOp::Ge } else { BinOp::Lt };
                    }
                }
            }
            // a copy of an array's elements from..to (a rest pattern's), as the array of them
            if let Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) = &*r {
                if let ([Projection::Subslice { from, to }], Some(a)) = (p.projection.as_slice(), p.base_local()) {
                    let element = |k: u64| {
                        Operand::Copy(Place {
                            base: PlaceBase::Local(a),
                            projection: vec![Projection::ConstantIndex {
                                offset: k,
                                min_length: *to,
                            }],
                        })
                    };
                    *r = Rvalue::Array((*from..*to).map(element).collect());
                }
            }
            if let Rvalue::Variant(p, args) = &*r {
                let of = match args.as_slice() {
                    [Operand::Copy(a) | Operand::Move(a)] => a.as_local(),
                    _ => None,
                };
                if let (true, Some(n)) = (p.is_path(&["PtrMetadata"]), of.and_then(|a| known.get(&a))) {
                    *r = Rvalue::Use(Operand::Constant(Const::Int {
                        value: *n as i128,
                        ty: IntTy::Usize,
                    }));
                }
            }
        }
    }
}

/// The locals that are copies of a slice iterator's own two pointers, `ptr` and `end_or_len` (or casts of them),
/// each with the iterator and the field it copies, 0 or 1: each assigned once, from the field or from another such
/// local.
fn slice_iter_fields(f: &Function, types: &HashMap<Local, Ty>) -> HashMap<Local, (Local, u32)> {
    let iter =
        |l: Local| matches!(types.get(&l), Some(Ty::Path(p)) if p.is_plain() && p.name().starts_with("slice_Iter"));
    let mut sets: HashMap<Local, usize> = HashMap::new();
    for d in f.body.blocks.iter().flat_map(|b| b.defs()).flatten() {
        *sets.entry(d).or_default() += 1;
    }
    let mut of: HashMap<Local, (Local, u32)> = HashMap::new();
    loop {
        let before = of.len();
        for s in f.body.blocks.iter().flat_map(|b| &b.statements) {
            let Statement::Assign(d, r) = s else { continue };
            let Some(l) = d.as_local().filter(|l| sets.get(l) == Some(&1) && !of.contains_key(l)) else {
                continue;
            };
            let field = match r {
                Rvalue::Use(Operand::Copy(p) | Operand::Move(p))
                | Rvalue::Cast(Operand::Copy(p) | Operand::Move(p), _, CastKind::Transmute | CastKind::PtrToPtr) => {
                    match (p.projection.as_slice(), p.base_local()) {
                        ([Projection::Field(k @ (0 | 1), _)], Some(it)) if iter(it) => Some((it, *k)),
                        ([], Some(q)) => of.get(&q).copied(),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(k) = field {
                of.insert(l, k);
            }
        }
        if of.len() == before {
            return of;
        }
    }
}

/// Where a pointer into an array starts: the array, a local of the function or behind a pointer that never
/// changes, and the index.
#[derive(Clone, PartialEq)]
enum Base {
    Array(Local),
    Behind(Local),
}

/// What a whole assignment to a pointer makes of it.
enum Def {
    /// `&t`, `&t[i]`, `&t[k]`: the array, and the index (a constant or a local).
    At(Base, Operand),
    /// `copy q`, or `Add(copy q, k)` (an `Offset`), of another pointer
    From(Local),
}

fn int_operand(o: &Operand) -> bool {
    matches!(o, Operand::Constant(Const::Int { .. }))
        || matches!(o, Operand::Copy(p) | Operand::Move(p) if p.as_local().is_some())
}

/// Pointers into one array, in a function: a pointer local whose every definition is the address of one of that
/// array's elements (or the array's own, its first), a copy of another such pointer, or one stepped by `Offset`,
/// and which is only compared with another, dereferenced (`*p`) or stepped, is the element's index instead, a
/// `usize`, and `*p` is `t[p]`. That is what the array's slice iterator comes to, once rustc has inlined it (and the
/// struct pass has taken its struct and its `Option<&T>` apart): a counted loop over the array, as `for i in 0..N`
/// with `t[i]` is, which the passes after know how to make small. A pointer that does anything else (passed to a
/// function, stored, returned, made from something else) keeps its address, and so do the pointers it is
/// compared with or made from.
pub fn pointers_as_indexes(f: &mut Function) {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    let element = |t: &Ty| matches!(t, Ty::Int(IntTy::I16 | IntTy::U8) | Ty::Bool);
    let candidate = |t: &Ty| match t {
        Ty::Ref { to, .. } | Ty::Ptr { to, .. } => {
            element(to) || matches!(&**to, Ty::Array(e, ArrayLen::Known(_)) if element(e))
        }
        _ => false,
    };
    let params: Vec<Local> = f.params.iter().map(|(l, _)| *l).collect();
    let pointers: Vec<Local> = f
        .body
        .decls
        .iter()
        .filter(|d| {
            candidate(&d.ty) && d.local != Local(0) && !params.contains(&d.local) && !f.results.contains(&d.local)
        })
        .map(|d| d.local)
        .collect();
    if pointers.is_empty() {
        return;
    }
    let is_ptr = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local().filter(|l| pointers.contains(l)),
        _ => None,
    };
    // a base behind a pointer: a parameter, or a local assigned once, neither ever written through
    let assigned = |l: Local| {
        f.body
            .blocks
            .iter()
            .flat_map(|b| b.defs())
            .filter(|d| *d == Some(l))
            .count()
    };
    let array_local = |l: Local| matches!(types.get(&l), Some(Ty::Array(e, ArrayLen::Known(_))) if element(e));
    // a pointer assigned once, the address of a whole array local (or a copy of such a pointer): that array,
    // wherever it is dereferenced
    let targets = whole_targets(f);
    let array_of = |l: Local| targets.get(&l).copied().filter(|a| array_local(*a));
    let base_of = |p: &Place| -> Option<(Base, Operand)> {
        let l = p.base_local()?;
        let (base, rest) = match p.projection.split_first() {
            Some((Projection::Deref, rest)) if array_of(l).is_some() => (Base::Array(array_of(l)?), rest),
            Some((Projection::Deref, rest)) => {
                let fixed = matches!(types.get(&l), Some(t) if candidate(t))
                    && !pointers.contains(&l)
                    && (params.contains(&l) && assigned(l) == 0 || !params.contains(&l) && assigned(l) == 1);
                if !fixed
                    || !matches!(types.get(&l), Some(Ty::Ref { to, .. } | Ty::Ptr { to, .. }) if matches!(&**to, Ty::Array(..)))
                {
                    return None;
                }
                (Base::Behind(l), rest)
            }
            _ if array_local(l) => (Base::Array(l), p.projection.as_slice()),
            _ => return None,
        };
        let index = match rest {
            [] => Operand::Constant(Const::Int {
                value: 0,
                ty: IntTy::Usize,
            }),
            [Projection::Index(i)] => Operand::Copy(Place::local(*i)),
            [Projection::ConstantIndex { offset, .. }] => Operand::Constant(Const::Int {
                value: *offset as i128,
                ty: IntTy::Usize,
            }),
            _ => return None,
        };
        Some((base, index))
    };
    // what each pointer's definitions and uses allow: the lines that name it, counted, against all of them
    let mut defs: HashMap<Local, Vec<Def>> = HashMap::new();
    let mut allowed: HashMap<Local, usize> = HashMap::new();
    let mut joined: Vec<(Local, Local)> = vec![];
    let mut bad: Vec<Local> = vec![];
    let deref = |p: &Place| match p.projection.as_slice() {
        [Projection::Deref] => p.base_local().filter(|l| pointers.contains(l)),
        _ => None,
    };
    // where each `&t[i]` is, and its index
    let mut made: Vec<(usize, usize, Local, Operand)> = vec![];
    for (bi, b) in f.body.blocks.iter().enumerate() {
        for (si, s) in b.statements.iter().enumerate() {
            match s {
                Statement::StorageLive(l) | Statement::StorageDead(l) => *allowed.entry(*l).or_default() += 1,
                Statement::Assign(d, r) => {
                    let dest = d.as_local().filter(|l| pointers.contains(l));
                    if let Some(p) = dest {
                        let def = match r {
                            Rvalue::Ref(_, q) => base_of(q).map(|(b, i)| Def::At(b, i)),
                            // (a copy of an array's pointer that never changes, a parameter say: its first)
                            Rvalue::Use(o) => is_ptr(o).map(Def::From).or_else(|| {
                                let whole = Place {
                                    base: PlaceBase::Local(o.local()?),
                                    projection: vec![Projection::Deref],
                                };
                                base_of(&whole).map(|(b, i)| Def::At(b, i))
                            }),
                            Rvalue::BinaryOp(BinOp::Add, o, k) if int_operand(k) => is_ptr(o).map(Def::From),
                            _ => None,
                        };
                        match def {
                            Some(def) => {
                                *allowed.entry(p).or_default() += 1;
                                if let Def::At(_, i) = &def {
                                    made.push((bi, si, p, i.clone()));
                                }
                                if let Def::From(q) = def {
                                    *allowed.entry(q).or_default() += 1;
                                    joined.push((p, q));
                                }
                                defs.entry(p).or_default().push(def);
                            }
                            None => bad.push(p),
                        }
                    }
                    if let Rvalue::BinaryOp(op, x, y) = r {
                        let compare = matches!(
                            op,
                            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                        );
                        if let (true, Some(p), Some(q)) = (compare, is_ptr(x), is_ptr(y)) {
                            *allowed.entry(p).or_default() += 1;
                            *allowed.entry(q).or_default() += 1;
                            joined.push((p, q));
                        }
                    }
                    if let Some(p) = deref(d) {
                        *allowed.entry(p).or_default() += 1;
                    }
                    if !matches!(r, Rvalue::Ref(..)) {
                        for p in r.operands().into_iter().filter_map(Operand::place).filter_map(deref) {
                            *allowed.entry(p).or_default() += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        for p in b
            .terminator
            .operands()
            .into_iter()
            .filter_map(Operand::place)
            .filter_map(deref)
        {
            *allowed.entry(p).or_default() += 1;
        }
    }
    // families: the pointers made from or compared with each other, which become indexes together or not at all
    let mut family: HashMap<Local, Local> = pointers.iter().map(|l| (*l, *l)).collect();
    fn root(family: &HashMap<Local, Local>, mut l: Local) -> Local {
        while family[&l] != l {
            l = family[&l];
        }
        l
    }
    for (p, q) in joined {
        let (a, b) = (root(&family, p), root(&family, q));
        family.insert(a, b);
    }
    // each family's array, or none where one of them can't be an index
    let mut bases: HashMap<Local, Option<Base>> = HashMap::new();
    let mut failed: Vec<Local> = vec![];
    for &p in &pointers {
        let r = root(&family, p);
        let fits = !bad.contains(&p)
            && defs.contains_key(&p)
            && allowed.get(&p).copied().unwrap_or(0) == ir::occurrences(&f.body.blocks, p);
        if !fits {
            failed.push(r);
        }
        for d in defs.get(&p).into_iter().flatten() {
            if let Def::At(b, _) = d {
                match bases.get(&r) {
                    Some(Some(c)) if c != b => failed.push(r),
                    _ => {
                        bases.insert(r, Some(b.clone()));
                    }
                }
            }
        }
    }
    let index: HashMap<Local, Base> = pointers
        .iter()
        .filter_map(|&p| {
            let r = root(&family, p);
            match bases.get(&r) {
                Some(Some(b)) if !failed.contains(&r) => Some((p, b.clone())),
                _ => None,
            }
        })
        .collect();
    if index.is_empty() {
        return;
    }
    for d in f.body.decls.iter_mut() {
        if index.contains_key(&d.local) {
            d.ty = Ty::Int(IntTy::Usize);
        }
    }
    let element_at = |b: &Base, i: Local| match b {
        Base::Array(t) => Place {
            base: PlaceBase::Local(*t),
            projection: vec![Projection::Index(i)],
        },
        Base::Behind(r) => Place {
            base: PlaceBase::Local(*r),
            projection: vec![Projection::Deref, Projection::Index(i)],
        },
    };
    for (bi, si, p, i) in made {
        if let (true, Statement::Assign(_, r)) = (index.contains_key(&p), &mut f.body.blocks[bi].statements[si]) {
            *r = Rvalue::Use(i);
        }
    }
    let indexes: Vec<Local> = index.keys().copied().collect();
    fold_indexes(f, &indexes);
    let rewrite = |p: &mut Place| {
        if let ([Projection::Deref], Some(l)) = (p.projection.as_slice(), p.base_local()) {
            if let Some(b) = index.get(&l) {
                *p = element_at(b, l);
            }
        }
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            s.places_mut(&mut |p| rewrite(p));
        }
        b.terminator.places_mut(&mut |p| rewrite(p));
    }
    forward_index_copies(f, &indexes);
    late_steps(f, &indexes);
}

/// The pointers assigned once that hold the address of a whole local: `p = &x` (or `&raw mut x`), or a copy of
/// another such pointer. Each with x.
fn whole_targets(f: &Function) -> HashMap<Local, Local> {
    let params: Vec<Local> = f.params.iter().map(|(l, _)| *l).collect();
    let mut sets: HashMap<Local, usize> = HashMap::new();
    for d in f.body.blocks.iter().flat_map(|b| b.defs()).flatten() {
        *sets.entry(d).or_default() += 1;
    }
    let mut target: HashMap<Local, Local> = HashMap::new();
    loop {
        let before = target.len();
        for s in f.body.blocks.iter().flat_map(|b| &b.statements) {
            let Statement::Assign(d, r) = s else { continue };
            let Some(p) = d.as_local().filter(|p| sets.get(p) == Some(&1) && !params.contains(p)) else {
                continue;
            };
            let x = match r {
                Rvalue::Ref(_, x) => x.as_local().filter(|x| !params.contains(x)),
                Rvalue::Use(Operand::Copy(q) | Operand::Move(q)) => q.as_local().and_then(|q| target.get(&q).copied()),
                _ => None,
            };
            if let Some(x) = x {
                target.insert(p, x);
            }
        }
        if target.len() == before {
            return target;
        }
    }
}

/// A pointer assigned once, the address of a whole scalar local x (or a copy of one), and used only as `*p` (read
/// or written) or copied into another such pointer: `*p` is x itself, and the pointers go. (A slice's `swap`
/// moves an element through a temporary so.)
pub fn pointers_to_locals(f: &mut Function) {
    let scalar = |t: &Ty| matches!(t, Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool);
    let types: HashMap<Local, Ty> = f
        .params
        .iter()
        .cloned()
        .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
        .collect();
    let targets: HashMap<Local, Local> = whole_targets(f)
        .into_iter()
        .filter(|(p, x)| {
            matches!(types.get(p), Some(Ty::Ref { .. } | Ty::Ptr { .. }))
                && types.get(x).is_some_and(scalar)
                && *p != Local(0)
                && !f.results.contains(p)
        })
        .collect();
    // every mention of each: its definition, `*p` as a whole place, a copy into another of them, its storage
    let fits = |p: Local| {
        let mut n = 0;
        for b in &f.body.blocks {
            for s in &b.statements {
                n += match s {
                    Statement::StorageLive(l) | Statement::StorageDead(l) if *l == p => 1,
                    Statement::Assign(d, _) if d.as_local() == Some(p) => 1,
                    Statement::Assign(d, Rvalue::Use(Operand::Copy(q) | Operand::Move(q)))
                        if q.as_local() == Some(p) && d.as_local().is_some_and(|d| targets.contains_key(&d)) =>
                    {
                        1
                    }
                    _ => 0,
                };
            }
            let places = b
                .statements
                .iter()
                .flat_map(|s| s.places())
                .chain(b.terminator.places());
            n += places
                .filter(|q| q.base_local() == Some(p) && q.projection.as_slice() == [Projection::Deref])
                .count();
        }
        n == ir::occurrences(&f.body.blocks, p)
    };
    // (all of a chain of copies, or none: a copy into one that can't go would be left reading nothing)
    let mut gone: Vec<Local> = targets.keys().copied().filter(|p| fits(*p)).collect();
    loop {
        let before = gone.len();
        let keep: Vec<Local> = gone
            .iter()
            .copied()
            .filter(|p| {
                f.body.blocks.iter().flat_map(|b| &b.statements).all(|s| match s {
                    Statement::Assign(d, Rvalue::Use(Operand::Copy(q) | Operand::Move(q)))
                        if q.as_local() == Some(*p) =>
                    {
                        d.as_local().is_some_and(|d| gone.contains(&d))
                    }
                    _ => true,
                })
            })
            .collect();
        gone = keep;
        if gone.len() == before {
            break;
        }
    }
    if gone.is_empty() {
        return;
    }
    for b in f.body.blocks.iter_mut() {
        b.statements.retain(|s| {
            !s.def().is_some_and(|d| gone.contains(&d))
                && !matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if gone.contains(l))
        });
        let mut to_x = |q: &mut Place| {
            if let (Some(p), [Projection::Deref]) = (q.base_local(), q.projection.as_slice()) {
                if gone.contains(&p) {
                    *q = Place::local(targets[&p]);
                }
            }
        };
        for s in b.statements.iter_mut() {
            s.places_mut(&mut to_x);
        }
        b.terminator.places_mut(&mut to_x);
    }
}

/// An index made of constants alone (`&t` is 0, the end pointer `&t` stepped by N is N): each read of it is the
/// constant, as a pointer's was the array's address. Only an index assigned once, so its value never changes.
fn fold_indexes(f: &mut Function, indexes: &[Local]) {
    let mut sets: HashMap<Local, usize> = HashMap::new();
    for d in f.body.blocks.iter().flat_map(|b| b.defs()).flatten() {
        *sets.entry(d).or_default() += 1;
    }
    let mut value: HashMap<Local, i128> = HashMap::new();
    loop {
        let before = value.len();
        for s in f.body.blocks.iter().flat_map(|b| &b.statements) {
            let Statement::Assign(d, r) = s else { continue };
            let Some(l) = d.as_local().filter(|l| indexes.contains(l) && sets.get(l) == Some(&1)) else {
                continue;
            };
            let of = |o: &Operand| match o {
                Operand::Constant(Const::Int { value, .. }) => Some(*value),
                Operand::Copy(p) | Operand::Move(p) => p.as_local().and_then(|q| value.get(&q).copied()),
                _ => None,
            };
            let v = match r {
                Rvalue::Use(o) => of(o),
                Rvalue::BinaryOp(BinOp::Add, a, b) => of(a).zip(of(b)).map(|(a, b)| a + b),
                _ => None,
            };
            if let Some(v) = v {
                value.insert(l, v);
            }
        }
        if value.len() == before {
            break;
        }
    }
    if value.is_empty() {
        return;
    }
    let mut fold = |o: &mut Operand| {
        if let Some(v) = o.local().and_then(|l| value.get(&l)) {
            *o = Operand::Constant(Const::Int {
                value: *v,
                ty: IntTy::Usize,
            });
        }
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            s.operands_mut().into_iter().for_each(&mut fold);
        }
        b.terminator.operands_mut().into_iter().for_each(&mut fold);
    }
}

/// The blocks reached from `from` (itself among them) without passing through `avoid`.
fn reached(blocks: &[Block], from: BasicBlock, avoid: BasicBlock) -> HashSet<BasicBlock> {
    let mut seen: HashSet<BasicBlock> = HashSet::from([from]);
    let mut work = vec![from];
    while let Some(n) = work.pop() {
        let Some(b) = blocks.iter().find(|b| b.name == n) else {
            continue;
        };
        for s in b.terminator.successors() {
            if s != avoid && seen.insert(s) {
                work.push(s);
            }
        }
    }
    seen
}

/// A slice iterator's loop, its pointers made indexes: the test copies the index (`x = copy p`) and the body steps
/// it first (`p = Add(copy x, const 1)`), reading x after. The step moves to the end of the body, where the loop
/// goes back to its test, and x is p itself, as the for-loop pass does for a range: the same loop as `while p < N
/// { ..t[p]..; p += 1; }`. Only where nothing the body leads to (but the test) reads p, so a `break` that leaves
/// it unstepped changes nothing, and x is read only in the test's block and what the body leads to.
fn late_steps(f: &mut Function, indexes: &[Local]) {
    let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let defs = |blocks: &[Block], l: Local| blocks.iter().flat_map(|b| b.defs()).filter(|d| *d == Some(l)).count();
    let mut tried: HashSet<(BasicBlock, Local)> = HashSet::new();
    loop {
        let blocks = &f.body.blocks;
        let preds = ir::predecessors(blocks);
        // (the step's block and line, the test's block and line, p and x)
        let mut found = None;
        'search: for (bi, b) in blocks.iter().enumerate() {
            for (si, s) in b.statements.iter().enumerate() {
                let Statement::Assign(d, Rvalue::BinaryOp(BinOp::Add, Operand::Copy(xp), one)) = s else {
                    continue;
                };
                let (Some(p), Some(x)) = (d.as_local(), xp.as_local()) else {
                    continue;
                };
                if !indexes.contains(&p) || !indexes.contains(&x) || !matches!(one.int(), Some((1, _))) {
                    continue;
                }
                if tried.contains(&(b.name, p)) {
                    continue;
                }
                let [h] = preds.get(&b.name).map(Vec::as_slice).unwrap_or(&[]) else {
                    continue;
                };
                let Some(hi) = blocks.iter().position(|y| y.name == *h) else {
                    continue;
                };
                let copy = |s: &Statement| copy_of(s) == Some((x, p));
                let Some(ci) = blocks[hi].statements.iter().position(copy) else {
                    continue;
                };
                if defs(blocks, x) != 1 {
                    continue;
                }
                found = Some((bi, si, hi, ci, p, x));
                break 'search;
            }
        }
        let Some((bi, si, hi, ci, p, x)) = found else { break };
        tried.insert((blocks[bi].name, p));
        let h = blocks[hi].name;
        let body = reached(blocks, blocks[bi].name, h);
        let latches: Vec<BasicBlock> = preds
            .get(&h)
            .into_iter()
            .flatten()
            .copied()
            .filter(|l| body.contains(l))
            .collect();
        let elsewhere = blocks.iter().enumerate().any(|(j, y)| {
            let inside = body.contains(&y.name);
            y.lines().enumerate().any(|(k, l)| {
                let ours = (j == bi && k == si) || (j == hi && k == ci);
                (inside && !ours && l.mentions(p)) || (!inside && j != hi && l.mentions(x))
            })
        });
        if latches.is_empty() || elsewhere {
            continue;
        }
        let step = Statement::Assign(
            Place::local(p),
            Rvalue::BinaryOp(
                BinOp::Add,
                Operand::Copy(Place::local(p)),
                Operand::Constant(Const::Int {
                    value: 1,
                    ty: IntTy::Usize,
                }),
            ),
        );
        f.body.blocks[bi].statements.remove(si);
        f.body.blocks[hi].statements.remove(ci);
        for l in latches {
            let li = f.body.blocks.iter().position(|y| y.name == l).unwrap();
            if f.body.blocks[li].terminator == Terminator::Goto(h) {
                f.body.blocks[li].statements.push(step.clone());
            } else {
                let fresh = BasicBlock(next_block);
                next_block += 1;
                for t in f.body.blocks[li].terminator.targets_mut() {
                    if *t == h {
                        *t = fresh;
                    }
                }
                f.body.blocks.push(Block {
                    name: fresh,
                    statements: vec![step.clone()],
                    terminator: Terminator::Goto(h),
                });
            }
        }
        for b in f.body.blocks.iter_mut() {
            b.statements
                .retain(|s| !matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if *l == x));
            for s in b.statements.iter_mut() {
                s.places_mut(&mut |q| rename(q, x, p));
            }
            b.terminator.places_mut(&mut |q| rename(q, x, p));
        }
    }
}

/// `x = copy p` of two whole locals: x and p.
fn copy_of(s: &Statement) -> Option<(Local, Local)> {
    match s {
        Statement::Assign(d, Rvalue::Use(Operand::Copy(q) | Operand::Move(q))) => Some((d.as_local()?, q.as_local()?)),
        _ => None,
    }
}

/// x is p, wherever a place names it.
fn rename(q: &mut Place, x: Local, p: Local) {
    if q.base == PlaceBase::Local(x) {
        q.base = PlaceBase::Local(p);
    }
    for pr in q.projection.iter_mut() {
        match pr {
            Projection::Index(i) | Projection::IndexPlus(i, _) if *i == x => *i = p,
            Projection::IndexPlace(inner) => rename(inner, x, p),
            _ => {}
        }
    }
}

/// The copies among the indexes that rustc's casts between pointer types left (`_32 = copy _26`), gone: an index
/// assigned once as a copy of another assigned once, where no read of it can come after the other changes but before
/// it is copied again, is the other; and one assigned once and read only by a copy into another, straight after, is
/// worked out there.
fn forward_index_copies(f: &mut Function, indexes: &[Local]) {
    loop {
        let blocks = &f.body.blocks;
        let mut sets: HashMap<Local, usize> = HashMap::new();
        for d in blocks.iter().flat_map(|b| b.defs()).flatten() {
            *sets.entry(d).or_default() += 1;
        }
        let once = |l: Local| indexes.contains(&l) && sets.get(&l) == Some(&1);
        let mut found = None;
        'search: for (bi, b) in blocks.iter().enumerate() {
            for (si, s) in b.statements.iter().enumerate() {
                let Some((a, c)) = copy_of(s) else { continue };
                if a == c || !indexes.contains(&a) || !indexes.contains(&c) {
                    continue;
                }
                // a = copy c, c's value worked out just before and read nowhere else: a is worked out there
                if si > 0 && once(c) && ir::occurrences(blocks, c) == 2 {
                    if let Statement::Assign(d, _) = &b.statements[si - 1] {
                        if d.as_local() == Some(c) {
                            found = Some((bi, si, a, c, true));
                            break 'search;
                        }
                    }
                }
                if once(a) && once(c) && steady(blocks, c, a) {
                    found = Some((bi, si, a, c, false));
                    break 'search;
                }
            }
        }
        let Some((bi, si, a, c, into)) = found else { return };
        let b = &mut f.body.blocks[bi];
        if into {
            let Statement::Assign(d, _) = &mut b.statements[si - 1] else {
                unreachable!()
            };
            *d = Place::local(a);
            b.statements.remove(si);
        } else {
            b.statements.remove(si);
            for b in f.body.blocks.iter_mut() {
                b.statements
                    .retain(|s| !matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if *l == a));
                for s in b.statements.iter_mut() {
                    s.places_mut(&mut |q| rename(q, a, c));
                }
                b.terminator.places_mut(&mut |q| rename(q, a, c));
            }
        }
    }
}

/// From where c (assigned once) is set, does every way to a read of a (assigned once, as a copy of c) pass a's own
/// assignment first?
fn steady(blocks: &[Block], c: Local, a: Local) -> bool {
    let at = |l: Local| {
        blocks
            .iter()
            .enumerate()
            .find_map(|(bi, b)| b.defs().iter().position(|d| *d == Some(l)).map(|i| (bi, i)))
    };
    let (Some((cb, ci)), Some((ab, ai))) = (at(c), at(a)) else {
        return false;
    };
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut work = vec![(cb, ci + 1)];
    while let Some((bi, from)) = work.pop() {
        if !seen.insert((bi, from)) {
            continue;
        }
        let b = &blocks[bi];
        let lines: Vec<&dyn ir::Locals> = b.lines().collect();
        let mut stopped = false;
        for (k, l) in lines.iter().enumerate().skip(from) {
            if bi == ab && k == ai {
                stopped = true;
                break;
            }
            if l.mentions(a) {
                return false;
            }
        }
        if !stopped {
            for s in b.terminator.successors() {
                if let Some(j) = blocks.iter().position(|y| y.name == s) {
                    work.push((j, 0));
                }
            }
        }
    }
    true
}
