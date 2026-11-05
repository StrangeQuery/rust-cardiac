//! Structs. CARDIAC has no notion of a record, so a struct is taken apart before anything else sees it: each
//! field of a struct local becomes a local of its own, and every later pass works on plain cells. For a struct
//! local _x of type S with fields f0, f1, ...:
//!   (_x.k: T)                 the field: a fresh local _y
//!   _x = S { f0: a, f1: b }   one assignment per field
//!   _a = copy _x              one copy per field (the same for move)
//!   f(move _x)                one argument per field; f's parameter is split the same way, in the same order
//!   _x = f(...)               f returns its fields in its own cells: the caller copies them out after the call
//! A field is a scalar (i16, u8, bool, usize), an array of scalars, or a struct, whose own fields are cells of the
//! struct it is in: a struct's cells are its leaves, `Line { a: Point, b: Point }`'s are a_x, a_y, b_x, b_y,
//! and a field that is a struct is a run of them. An array of structs is kept as one array per leaf:
//! points[i].x is points_x[i], so indexing works as for any array. A struct behind a shared reference is the struct
//! by value; behind a `&mut`, a `&mut` of each leaf ((*r).x is *r_x). A tuple of scalars that crosses a call is a
//! struct too.
use std::collections::HashMap;

use super::ir::{
    ArrayLen, BasicBlock, BinOp, Block, Body, BorrowKind, Const, Debug, Decl, File, IntTy, Local, Operand, Path, Place,
    PlaceBase, Projection, Rvalue, Segment, Statement, Terminator, Ty,
};
use super::util::Ordered;
use super::Fail;

/// A function as the passes after the for loops work on it: its name, parameters (each local and its type), return
/// type, body, and, when it returns a struct, the locals that hold the struct's fields.
pub struct Function {
    pub name: String,
    pub params: Vec<(Local, Ty)>,
    pub ret: Ty,
    pub body: Body,
    pub results: Vec<Local>,
}

/// A field of a struct: its name and type.
#[derive(Clone)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
}

/// A struct local (n == 0), an array of n structs, or a struct behind a `&mut` (by_ref), and the fresh locals
/// for its leaves: a leaf's value, the array of its values, or a `&mut` of it.
#[derive(Clone)]
struct Split {
    strukt: String,
    n: usize,
    by_ref: bool,
    cells: Vec<Local>,
}

/// A path's struct name, as the pass knows structs: the segment that carries the generic arguments, else the last
/// (`core::ops::Range<i16>` is `Range`).
fn struct_name(p: &Path) -> String {
    let seg = p
        .segments
        .iter()
        .find(|s| matches!(s, Segment::Name { args: Some(_), .. }))
        .or(p.segments.last());
    match seg {
        Some(Segment::Name { name, .. }) => name.clone(),
        _ => String::new(),
    }
}

/// A plain path, with no generic arguments: the kind a struct literal the pass takes apart has.
fn plain_path(p: &Path) -> bool {
    p.qself.is_none() && p.segments.iter().all(|s| matches!(s, Segment::Name { args: None, .. }))
}

/// An index of some kind: `[_3]`, `[_3+1]` or `[2 of 3]`.
fn indexing(p: &Projection) -> bool {
    matches!(
        p,
        Projection::Index(_) | Projection::IndexPlus(..) | Projection::ConstantIndex { .. } | Projection::IndexPlace(_)
    )
}

/// Take every struct local apart, in every function. `file` is the whole program's MIR, where the structs not built
/// in these functions are found too; `known` are structs given with their fields (the enums with data, as
/// `enums::data_enums` writes them).
pub fn scalarize_structs(
    fns: &mut Ordered<String, Function>,
    file: &File,
    known: Vec<(String, Vec<Field>)>,
) -> Result<(), Fail> {
    // the structs: their fields in declaration order, from the aggregates that build them
    shared_scalar_refs_by_value(fns);
    let mut layouts: HashMap<String, Vec<Field>> = known.into_iter().collect();
    layouts.extend(tuples_as_structs(fns));
    let mut learn = |p: &Path, names: Vec<&String>| {
        if !plain_path(p) {
            return;
        }
        let name = struct_name(p);
        layouts.entry(name).or_insert_with(|| {
            names
                .into_iter()
                .map(|n| Field {
                    name: n.clone(),
                    ty: Ty::Int(IntTy::I16),
                })
                .collect()
        });
    };
    for (_, f) in fns.iter() {
        for b in &f.body.blocks {
            for s in &b.statements {
                if let Statement::Assign(_, Rvalue::Struct(p, fields)) = s {
                    learn(p, fields.iter().map(|(n, _)| n).collect());
                }
            }
            // a struct written out as an operand: a named const inlined, or a literal rustc folded into one constant
            // (`field(const Pair {{ a: 25_i16, b: 200_i16 }})`)
            let operands = b
                .statements
                .iter()
                .flat_map(|s| s.operands())
                .chain(b.terminator.operands());
            for o in operands {
                match o {
                    Operand::Aggregate(p, fields) => learn(p, fields.iter().map(|(n, _)| n).collect()),
                    Operand::Constant(Const::Adt(p, fields)) if fields.iter().all(|(_, c)| c.is_scalar()) => {
                        learn(p, fields.iter().map(|(n, _)| n).collect())
                    }
                    _ => {}
                }
            }
        }
    }
    // and in the rest of the MIR (const items, functions this compiler does not read)
    for s in file.bodies().flat_map(|b| &b.blocks).flat_map(|b| &b.statements) {
        if let Statement::Assign(_, Rvalue::Struct(p, fields)) = s {
            learn(p, fields.iter().map(|(n, _)| n).collect());
        }
    }
    if layouts.is_empty() {
        return Ok(());
    }
    let mut shapes = Shapes {
        layouts,
        leaves: HashMap::new(),
    };
    // a struct behind a shared reference is the struct itself, by value
    let names: Vec<String> = fns.keys().cloned().collect();
    for n in &names {
        shared_refs_by_value(fns.get_mut(n).unwrap(), &|t| shapes.strukt(t).is_some());
    }
    // the fields' types: from the places that read or write them, and from the structs built of whole values
    for (_, f) in fns.iter() {
        let types = local_types(f);
        for b in &f.body.blocks {
            for p in b
                .statements
                .iter()
                .flat_map(|s| s.places())
                .chain(b.terminator.places())
            {
                shapes.learn_place(&f.name, p, &types)?;
            }
            for s in &b.statements {
                if let Statement::Assign(_, Rvalue::Struct(p, fs)) = s {
                    shapes.learn_built(&f.name, p, fs, &types)?;
                }
            }
            let operands = b
                .statements
                .iter()
                .flat_map(|s| s.operands())
                .chain(b.terminator.operands());
            for o in operands {
                if let Operand::Aggregate(p, fs) = o {
                    shapes.learn_built(&f.name, p, fs, &types)?;
                }
            }
        }
    }
    shapes.flatten();
    let shapes = &shapes;
    // each function: which locals are structs (or arrays of structs, or `&mut` structs), and the fresh locals for
    // their leaves
    let mut split: HashMap<String, Ordered<Local, Split>> = HashMap::new();
    for (_, f) in fns.iter() {
        let mut locals: Ordered<Local, Ty> = Ordered::new();
        for (l, t) in &f.params {
            locals.set(*l, t.clone());
        }
        for d in &f.body.decls {
            locals.set(d.local, d.ty.clone());
        }
        locals.set(Local(0), f.ret.clone());
        let mut next = locals.keys().map(|l| l.0).max().unwrap_or(0) + 1;
        let mut fields: Ordered<Local, Split> = Ordered::new();
        for (l, ty) in locals.iter() {
            let mut fresh = |s: &str| {
                (0..shapes.leaves[s].len())
                    .map(|_| {
                        let c = Local(next);
                        next += 1;
                        c
                    })
                    .collect::<Vec<_>>()
            };
            let has_array = |s: &str| shapes.leaves[s].iter().any(|x| matches!(x.ty, Ty::Array(..)));
            if let Ty::Ref { to, .. } | Ty::Ptr { to, .. } = ty {
                let inner = match &**to {
                    Ty::Array(e, _) => &**e,
                    t => t,
                };
                let Some(s) = shapes.strukt(inner) else { continue };
                // a `&mut` struct is a `&mut` of each leaf
                let by_ref = matches!(ty, Ty::Ref { mutable: true, to, .. } if shapes.strukt(to).is_some());
                if !by_ref || *l == Local(0) || has_array(&s) {
                    return Err(format!(
                        "{}: {l}: {ty}: a struct behind a reference is not supported yet",
                        f.name
                    ));
                }
                let cells = fresh(&s);
                fields.set(
                    *l,
                    Split {
                        strukt: s,
                        n: 0,
                        by_ref: true,
                        cells,
                    },
                );
            } else if let Some(s) = shapes.strukt(ty) {
                let cells = fresh(&s);
                fields.set(
                    *l,
                    Split {
                        strukt: s,
                        n: 0,
                        by_ref: false,
                        cells,
                    },
                );
            } else if let Ty::Array(e, ArrayLen::Known(n)) = ty {
                if let Some(s) = shapes.strukt(e) {
                    if has_array(&s) {
                        return Err(format!(
                            "{}: {l}: {ty}: an array of structs with arrays in them is not supported yet",
                            f.name
                        ));
                    }
                    let cells = fresh(&s);
                    fields.set(
                        *l,
                        Split {
                            strukt: s,
                            n: *n as usize,
                            by_ref: false,
                            cells,
                        },
                    );
                }
            }
        }
        split.insert(f.name.clone(), fields);
    }
    let names: Vec<String> = fns.keys().cloned().collect();
    for fname in names {
        let fields = split.get(&fname).unwrap().clone();
        let ty = |x: &Split, k: usize| {
            let t = shapes.leaves[&x.strukt][k].ty.clone();
            if x.by_ref {
                Ty::Ref {
                    lifetime: None,
                    mutable: true,
                    to: Box::new(t),
                }
            } else if x.n > 0 {
                Ty::Array(Box::new(t), ArrayLen::Known(x.n as u64))
            } else {
                t
            }
        };
        let field_name = |x: &Split, k: usize| shapes.leaves[&x.strukt][k].name.clone();
        let f = fns.get_mut(&fname).unwrap();
        // parameters and the result
        let mut params = vec![];
        for (l, t) in &f.params {
            match fields.get(l) {
                Some(x) if x.n > 0 => {
                    return Err(format!(
                        "{fname}: an array of structs as a parameter is not supported yet"
                    ))
                }
                Some(x) => params.extend(x.cells.iter().enumerate().map(|(k, c)| (*c, ty(x, k)))),
                None => params.push((*l, t.clone())),
            }
        }
        f.params = params;
        let result = fields.get(&Local(0)).cloned();
        if let Some(r) = &result {
            f.ret = Ty::Unit;
            f.results = r.cells.clone();
        }
        // declarations: the struct local goes, its leaves come in, named after the variable (p_x, p_y, ...)
        let mut decls = vec![];
        for d in &f.body.decls {
            match fields.get(&d.local) {
                Some(x) => decls.extend(x.cells.iter().enumerate().map(|(k, c)| Decl {
                    local: *c,
                    mutable: d.mutable,
                    ty: ty(x, k),
                })),
                None => decls.push(d.clone()),
            }
        }
        f.body.decls = decls;
        let mut debug = vec![];
        // the result's leaves are read by the caller after the call: name them, so no pass treats them as dead
        if let Some(r) = &result {
            debug.extend(r.cells.iter().enumerate().map(|(k, c)| Debug {
                name: format!("result_{}", field_name(r, k)),
                value: c.to_string(),
            }));
        }
        for d in &f.body.debug {
            match d.named_local().and_then(|(name, l)| fields.get(&l).map(|x| (name, x))) {
                Some((name, x)) => debug.extend(x.cells.iter().enumerate().map(|(k, c)| Debug {
                    name: format!("{name}_{}", field_name(x, k)),
                    value: c.to_string(),
                })),
                None => debug.push(d.clone()),
            }
        }
        f.body.debug = debug;
        // a leaf's projections: of a struct, (_x.k: T); of an element, (_a[i].k: T); through a `&mut`, ((*_r).k: T)
        fn project(p: &mut Place, fields: &Ordered<Local, Split>, shapes: &Shapes) {
            for q in p.projection.iter_mut() {
                if let Projection::IndexPlace(inner) = q {
                    project(inner, fields, shapes);
                    // the index is a local now: an ordinary index
                    if let Some(l) = inner.as_local() {
                        *q = Projection::Index(l);
                    }
                }
            }
            if let Some(At::Leaf(c)) = resolve(p, fields, shapes) {
                *p = c;
            }
        }
        let project = &mut |p: &mut Place| project(p, &fields, shapes);
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.places_mut(project);
            }
            b.terminator.places_mut(project);
        }
        // the statements, leaf by leaf
        let mut extra = vec![]; // new blocks: a struct result copied out after its call
        let mut nb = 900000;
        for bi in 0..f.body.blocks.len() {
            let mut statements = vec![];
            for s in &f.body.blocks[bi].statements {
                match split_statement(&fname, s, &fields, shapes)? {
                    Some(v) => statements.extend(v),
                    None => statements.push(s.clone()),
                }
            }
            let terminator = split_call(
                &fname,
                &f.body.blocks[bi].terminator,
                &fields,
                &split,
                shapes,
                &mut extra,
                &mut nb,
            )?;
            let b = &mut f.body.blocks[bi];
            b.statements = statements;
            b.terminator = terminator;
        }
        f.body.blocks.extend(extra);
    }
    Ok(())
}

/// A shared reference to a number or a bool (`fn eq(a: &Colour, b: &Colour)`, as a derived `==` takes them),
/// which can't change while it lives, is the value itself: no address passed, no read through it. Only where every
/// reference it meets is so too: a reference is taken as a value when each use of it is a read through it (`*r`) or
/// an argument to a function whose parameter is taken so, and each definition is a borrow of a place (`r = &x`, a
/// copy of x then) or of another such reference. One used otherwise (cast to a pointer, say) stays a reference, and
/// with it every parameter it is passed to, and every argument passed to such a parameter.
fn shared_scalar_refs_by_value(fns: &mut Ordered<String, Function>) {
    let scalar_ref = |t: &Ty| matches!(t, Ty::Ref { mutable: false, to, .. } if matches!(**to, Ty::Int(_) | Ty::Bool));
    // (function, local)
    let mut ok: Vec<(String, Local)> = vec![];
    for (name, f) in fns.iter() {
        for (l, t) in &f.params {
            if scalar_ref(t) {
                ok.push((name.clone(), *l));
            }
        }
        for d in &f.body.decls {
            if scalar_ref(&d.ty) {
                ok.push((name.clone(), d.local));
            }
        }
    }
    if ok.is_empty() {
        return;
    }
    let is = |ok: &Vec<(String, Local)>, f: &str, l: Local| ok.iter().any(|(g, m)| g == f && *m == l);
    loop {
        let mut drop: Vec<(String, Local)> = vec![];
        for (name, f) in fns.iter() {
            let mine = |l: Local| is(&ok, name, l);
            for b in &f.body.blocks {
                for s in &b.statements {
                    // its definitions: a borrow of a place with no index, or a copy of another
                    if let Statement::Assign(d, rv) = s {
                        if let Some(r) = d.as_local().filter(|r| mine(*r)) {
                            let fine = match rv {
                                Rvalue::Ref(_, p) => {
                                    p.base_local().is_some()
                                        && p.projection.iter().all(|x| matches!(x, Projection::Field(..)))
                                }
                                Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => p.as_local().is_some_and(mine),
                                _ => false,
                            };
                            if !fine {
                                drop.push((name.clone(), r));
                            }
                        }
                    }
                    // its uses: reads through it, `*r`
                    for p in s.places() {
                        if let Some(r) = p.base_local().filter(|r| mine(*r)) {
                            let read = p.projection.as_slice() == [Projection::Deref];
                            let defined = matches!(s, Statement::Assign(d, _) if d == p);
                            let copied = matches!(s, Statement::Assign(_, Rvalue::Use(Operand::Copy(q) | Operand::Move(q))) if q == p && p.projection.is_empty());
                            if !(read || defined || copied) {
                                drop.push((name.clone(), r));
                            }
                        }
                    }
                }
                match &b.terminator {
                    Terminator::Call { dest, func, args, .. } => {
                        if let Some(r) = dest.base_local().filter(|r| mine(*r)) {
                            drop.push((name.clone(), r));
                        }
                        let callee = func.to_string();
                        let theirs = fns
                            .get(&callee)
                            .map(|g| g.params.iter().map(|(l, _)| *l).collect::<Vec<_>>());
                        for (k, a) in args.iter().enumerate() {
                            for p in a.place().into_iter() {
                                let Some(r) = p.base_local() else { continue };
                                let param = theirs.as_ref().and_then(|ps| ps.get(k)).copied();
                                let read = p.projection.as_slice() == [Projection::Deref];
                                if mine(r) && !read {
                                    // passed whole: only to a parameter taken as a value, and that only from such an
                                    // argument
                                    let passed = p.projection.is_empty() && param.is_some_and(|q| is(&ok, &callee, q));
                                    if !passed {
                                        drop.push((name.clone(), r));
                                    }
                                }
                                if let Some(q) = param.filter(|q| is(&ok, &callee, *q)) {
                                    if !(mine(r) && p.projection.is_empty()) {
                                        drop.push((callee.clone(), q));
                                    }
                                }
                            }
                            // a constant passed to such a parameter (no place): it stays a reference
                            if a.place().is_none() {
                                if let Some(q) = theirs.as_ref().and_then(|ps| ps.get(k)).copied() {
                                    if is(&ok, &callee, q) {
                                        drop.push((callee.clone(), q));
                                    }
                                }
                            }
                        }
                    }
                    t => {
                        for p in t.places() {
                            if let Some(r) = p.base_local().filter(|r| mine(*r)) {
                                if p.projection.as_slice() != [Projection::Deref] {
                                    drop.push((name.clone(), r));
                                }
                            }
                        }
                    }
                }
            }
            // a function returning one: not a value
            if scalar_ref(&f.ret) {
                drop.push((name.clone(), Local(0)));
            }
        }
        let before = ok.len();
        ok.retain(|x| !drop.contains(x));
        if ok.len() == before {
            break;
        }
    }
    if ok.is_empty() {
        return;
    }
    let names: Vec<String> = fns.keys().cloned().collect();
    for name in names {
        let mine: Vec<Local> = ok.iter().filter(|(g, _)| *g == name).map(|(_, l)| *l).collect();
        if mine.is_empty() {
            continue;
        }
        let f = fns.get_mut(&name).unwrap();
        let unref = |t: &mut Ty| {
            if let Ty::Ref { to, .. } = t {
                *t = (**to).clone();
            }
        };
        for (l, t) in f.params.iter_mut() {
            if mine.contains(l) {
                unref(t);
            }
        }
        for d in f.body.decls.iter_mut() {
            if mine.contains(&d.local) {
                unref(&mut d.ty);
            }
        }
        let mut strip = |p: &mut Place| {
            if p.base_local().is_some_and(|l| mine.contains(&l)) && p.projection.first() == Some(&Projection::Deref) {
                p.projection.remove(0);
            }
        };
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.places_mut(&mut strip);
                if let Statement::Assign(d, r @ Rvalue::Ref(..)) = s {
                    if let (Some(l), Rvalue::Ref(_, p)) = (d.as_local(), &*r) {
                        if mine.contains(&l) {
                            *r = Rvalue::Use(Operand::Copy(p.clone()));
                        }
                    }
                }
            }
            b.terminator.places_mut(&mut strip);
        }
    }
}

/// A tuple of scalars that crosses a call (`fn split(v: i16) -> (i16, i16)`, rustc takes the others apart itself) is
/// a struct whose fields are `.0`, `.1`, ..., which the struct pass then takes apart as any: its type a
/// struct's named after its elements' (`tuple__i16_i16`), and `(a, b)` that struct built. An overflow-checked op's
/// pair (`AddWithOverflow`'s `(i16, bool)`) stays a tuple: the passes after know it so. Returns the structs, each with
/// its fields.
fn tuples_as_structs(fns: &mut Ordered<String, Function>) -> Vec<(String, Vec<Field>)> {
    let mut made: Vec<(String, Vec<Field>)> = vec![];
    let names: Vec<String> = fns.keys().cloned().collect();
    for n in &names {
        let f = fns.get_mut(n).unwrap();
        // the overflow-checked ops' pairs
        let pairs: Vec<Local> = f
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .filter_map(|s| match s {
                Statement::Assign(
                    d,
                    Rvalue::BinaryOp(BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow, ..),
                ) => d.as_local(),
                _ => None,
            })
            .collect();
        let mut strukt = |t: &Ty| -> Option<Ty> {
            let Ty::Tuple(ts) = t else { return None };
            let scalar = |t: &Ty| matches!(t, Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool);
            if ts.len() < 2 || !ts.iter().all(scalar) {
                return None;
            }
            let name = format!(
                "tuple__{}",
                ts.iter().map(|t| t.to_string()).collect::<Vec<_>>().join("_")
            );
            if !made.iter().any(|(m, _)| *m == name) {
                let fields = ts
                    .iter()
                    .enumerate()
                    .map(|(k, t)| Field {
                        name: k.to_string(),
                        ty: t.clone(),
                    })
                    .collect();
                made.push((name.clone(), fields));
            }
            Some(Ty::Path(Path {
                qself: None,
                segments: vec![Segment::Name { name, args: None }],
            }))
        };
        let mut tuples: Vec<(Local, Path, usize)> = vec![];
        let mut retype = |l: Local, t: &mut Ty, tuples: &mut Vec<(Local, Path, usize)>| {
            if pairs.contains(&l) {
                return;
            }
            let n = match t {
                Ty::Tuple(ts) => ts.len(),
                _ => return,
            };
            if let Some(s) = strukt(t) {
                let Ty::Path(p) = &s else { unreachable!() };
                tuples.push((l, p.clone(), n));
                *t = s;
            }
        };
        for (l, t) in f.params.iter_mut() {
            retype(*l, t, &mut tuples);
        }
        retype(Local(0), &mut f.ret, &mut tuples);
        for d in f.body.decls.iter_mut() {
            retype(d.local, &mut d.ty, &mut tuples);
        }
        if tuples.is_empty() {
            continue;
        }
        // `(a, b)` into one of them: the struct built
        for s in f.body.blocks.iter_mut().flat_map(|b| b.statements.iter_mut()) {
            if let Statement::Assign(d, r @ Rvalue::Tuple(_)) = s {
                let Some((_, p, n)) = d.as_local().and_then(|l| tuples.iter().find(|(t, ..)| *t == l)) else {
                    continue;
                };
                let Rvalue::Tuple(ops) = &*r else { unreachable!() };
                if ops.len() == *n {
                    *r = Rvalue::Struct(
                        p.clone(),
                        ops.iter()
                            .enumerate()
                            .map(|(k, o)| (k.to_string(), o.clone()))
                            .collect(),
                    );
                }
            }
        }
    }
    made
}

/// A struct behind a shared reference (`fn show(p: &Pair)`, `let r = &q;`), which can't change while the reference
/// lives, is the struct by value: the reference's type is the struct's, `(*r).k` is `r.k`, and `r = &p` is a copy
/// of p. (`&mut` stays refused: a write through it must reach the struct it borrows.)
fn shared_refs_by_value(f: &mut Function, is_struct: &dyn Fn(&Ty) -> bool) {
    let shared = |t: &Ty| matches!(t, Ty::Ref { mutable: false, to, .. } if is_struct(to));
    let mut by_value: Vec<Local> = f.params.iter().filter(|(_, t)| shared(t)).map(|(l, _)| *l).collect();
    by_value.extend(f.body.decls.iter().filter(|d| shared(&d.ty)).map(|d| d.local));
    if shared(&f.ret) {
        by_value.push(Local(0));
    }
    if by_value.is_empty() {
        return;
    }
    let unref = |t: &mut Ty| {
        if let Ty::Ref { to, .. } = t {
            *t = (**to).clone();
        }
    };
    for (l, t) in f.params.iter_mut() {
        if by_value.contains(l) {
            unref(t);
        }
    }
    for d in f.body.decls.iter_mut() {
        if by_value.contains(&d.local) {
            unref(&mut d.ty);
        }
    }
    if by_value.contains(&Local(0)) {
        unref(&mut f.ret);
    }
    let mut strip = |p: &mut Place| {
        if p.base_local().is_some_and(|l| by_value.contains(&l)) && p.projection.first() == Some(&Projection::Deref) {
            p.projection.remove(0);
        }
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            s.places_mut(&mut strip);
            if let Statement::Assign(d, r @ Rvalue::Ref(..)) = s {
                if let (Some(l), Rvalue::Ref(_, p)) = (d.as_local(), &*r) {
                    if by_value.contains(&l) {
                        *r = Rvalue::Use(Operand::Copy(p.clone()));
                    }
                }
            }
        }
        b.terminator.places_mut(&mut strip);
    }
}

/// The structs the pass takes apart: each one's fields, and, once their types are known, its leaves: its fields
/// with each struct field in turn its own leaves (`Card { n, kind: Option<i16> }` is `n`, `kind_tag`, `kind_Some_0`),
/// each leaf one cell, or an array of scalars.
struct Shapes {
    layouts: HashMap<String, Vec<Field>>,
    leaves: HashMap<String, Vec<Field>>,
}

impl Shapes {
    /// The struct a type is, if it is one the pass takes apart.
    fn strukt(&self, ty: &Ty) -> Option<String> {
        let Ty::Path(p) = ty else { return None };
        let s = struct_name(p);
        self.layouts.contains_key(&s).then_some(s)
    }

    /// Field k of struct s is of type t: a scalar, a struct, or an array of scalars.
    fn set(&mut self, fname: &str, s: &str, k: u32, t: &Ty) -> Result<(), Fail> {
        let scalar = |t: &Ty| matches!(t, Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool);
        let fine = scalar(t)
            || matches!(t, Ty::Array(e, ArrayLen::Known(_)) if scalar(e))
            || self.strukt(t).is_some_and(|inner| inner != s);
        // (a type the pass never followed, a reference or a generic path, is left as it was)
        let one_word = match t {
            Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str => true,
            Ty::Path(p) => p.qself.is_none() && matches!(p.segments.as_slice(), [Segment::Name { args: None, .. }]),
            _ => false,
        };
        if !fine && !one_word {
            return Ok(());
        }
        if !fine {
            return Err(format!(
                "{fname}: field {k} of {s} is {t}: fields must be scalars, structs or arrays of scalars"
            ));
        }
        let layout = self.layouts.get_mut(s).unwrap();
        if k as usize >= layout.len() {
            return Err(format!("{fname}: field {k} of {s} is past its last field"));
        }
        layout[k as usize].ty = t.clone();
        Ok(())
    }

    /// The types of the struct fields a place goes through, `((_4.1: Option__enum0).0: i16)`, and those its
    /// indexes do (`t[(_2.0: usize)]`).
    fn learn_place(&mut self, fname: &str, p: &Place, types: &HashMap<Local, Ty>) -> Result<(), Fail> {
        for x in &p.projection {
            if let Projection::IndexPlace(inner) = x {
                self.learn_place(fname, inner, types)?;
            }
        }
        let Some(mut t) = p.base_local().and_then(|l| types.get(&l)).cloned() else {
            return Ok(());
        };
        for x in &p.projection {
            t = match (x, t) {
                (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
                (x, Ty::Array(e, _) | Ty::Slice(e)) if indexing(x) => *e,
                (Projection::Field(k, ft), t) => {
                    if let Some(s) = self.strukt(&t) {
                        self.set(fname, &s, *k, ft)?;
                    }
                    ft.clone()
                }
                _ => break,
            };
        }
        Ok(())
    }

    /// The types of a struct's fields that are built of whole structs or arrays, `Card { n: copy _1, kind: move _2 }`.
    fn learn_built(
        &mut self,
        fname: &str,
        p: &Path,
        fs: &[(String, Operand)],
        types: &HashMap<Local, Ty>,
    ) -> Result<(), Fail> {
        if !plain_path(p) || !self.layouts.contains_key(&struct_name(p)) {
            return Ok(());
        }
        for (k, (_, o)) in fs.iter().enumerate() {
            let t = o.place().and_then(|q| q.as_local()).and_then(|l| types.get(&l));
            if let Some(t) = t.filter(|t| matches!(t, Ty::Array(..)) || self.strukt(t).is_some()) {
                self.set(fname, &struct_name(p), k as u32, t)?;
            }
        }
        Ok(())
    }

    /// Each struct's leaves, once its fields' types are known.
    fn flatten(&mut self) {
        fn leaves(s: &str, shapes: &Shapes) -> Vec<Field> {
            let mut out = vec![];
            for f in &shapes.layouts[s] {
                match shapes.strukt(&f.ty) {
                    Some(inner) => out.extend(leaves(&inner, shapes).into_iter().map(|x| Field {
                        name: format!("{}_{}", f.name, x.name),
                        ty: x.ty,
                    })),
                    None => out.push(f.clone()),
                }
            }
            out
        }
        self.leaves = self.layouts.keys().map(|s| (s.clone(), leaves(s, self))).collect();
    }

    /// Where field k of struct s starts among its leaves, how many it has, and its type.
    fn offset(&self, s: &str, k: usize) -> Option<(usize, usize, &Ty)> {
        let width = |t: &Ty| self.strukt(t).map_or(1, |inner| self.leaves[&inner].len());
        let fs = self.layouts.get(s)?;
        let t = &fs.get(k)?.ty;
        Some((fs[..k].iter().map(|f| width(&f.ty)).sum(), width(t), t))
    }
}

/// Each local's type in a function: its parameters', its declarations', and the result's.
fn local_types(f: &Function) -> HashMap<Local, Ty> {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    types.insert(Local(0), f.ret.clone());
    types
}

/// What a place within a struct local is: one leaf's cell, with the projections that follow it (an index into an
/// array leaf), or a whole struct, the place of each of its leaves.
enum At {
    Leaf(Place),
    Whole(Vec<Place>),
}

/// The place `p` within a struct local, an element of an array of structs, or a struct behind a `&mut` (where the
/// reference itself, `_r`, is the whole of its leaves' references).
fn resolve(p: &Place, fields: &Ordered<Local, Split>, shapes: &Shapes) -> Option<At> {
    let sp = fields.get(&p.base_local()?)?;
    let proj = &p.projection;
    let mut pre = vec![];
    if sp.by_ref {
        if proj.is_empty() {
            return Some(At::Whole(sp.cells.iter().map(|c| cell(*c, &[])).collect()));
        }
        if proj[0] != Projection::Deref {
            return None;
        }
        pre.push(Projection::Deref);
    }
    if sp.n > 0 {
        pre.push(proj.get(pre.len()).filter(|q| indexing(q))?.clone());
    }
    let mut i = pre.len();
    let (mut lo, mut hi) = (0, sp.cells.len());
    let mut s = sp.strukt.clone();
    while let Some(Projection::Field(k, _)) = proj.get(i) {
        let (start, width, t) = shapes.offset(&s, *k as usize)?;
        lo += start;
        hi = lo + width;
        i += 1;
        match shapes.strukt(t) {
            Some(inner) => s = inner,
            None => {
                let mut c = cell(sp.cells[lo], &pre);
                c.projection.extend(proj[i..].iter().cloned());
                return Some(At::Leaf(c));
            }
        }
    }
    (i == proj.len()).then(|| At::Whole(sp.cells[lo..hi].iter().map(|c| cell(*c, &pre)).collect()))
}

/// A place on a field's cell, with the projections the struct's place had (an element's index).
fn cell(c: Local, projection: &[Projection]) -> Place {
    Place {
        base: PlaceBase::Local(c),
        projection: projection.to_vec(),
    }
}

/// A struct-valued operand as one operand per leaf: a struct local, an element of an array of structs, a struct
/// field, a struct behind a `&mut` (their leaves as places), or a struct written out.
fn ops_of(o: &Operand, fields: &Ordered<Local, Split>, shapes: &Shapes) -> Option<Vec<Operand>> {
    match o {
        Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => {
            // (a bare place is no whole struct: an element of an array of structs is)
            if matches!(o, Operand::Place(_)) && p.projection.is_empty() {
                return None;
            }
            let At::Whole(ps) = resolve(p, fields, shapes)? else {
                return None;
            };
            let moved = matches!(o, Operand::Move(_));
            Some(
                ps.into_iter()
                    .map(|pl| if moved { Operand::Move(pl) } else { Operand::Copy(pl) })
                    .collect(),
            )
        }
        Operand::Aggregate(p, fs) if plain_path(p) => built(p, fs.iter().map(|(_, o)| o.clone()), fields, shapes),
        // a struct literal rustc folded into one constant: its fields' constants
        Operand::Constant(Const::Adt(p, fs)) if plain_path(p) => {
            built(p, fs.iter().map(|(_, c)| Operand::Constant(c.clone())), fields, shapes)
        }
        _ => None,
    }
}

/// A struct built of its fields' values as one operand per leaf: each field that is a struct, its own leaves.
fn built(
    p: &Path,
    values: impl Iterator<Item = Operand>,
    fields: &Ordered<Local, Split>,
    shapes: &Shapes,
) -> Option<Vec<Operand>> {
    let layout = shapes.layouts.get(&struct_name(p));
    let mut out = vec![];
    for (k, o) in values.enumerate() {
        match layout.and_then(|l| l.get(k)).and_then(|f| shapes.strukt(&f.ty)) {
            Some(inner) => {
                let ops = ops_of(&o, fields, shapes)?;
                if ops.len() != shapes.leaves[&inner].len() {
                    return None;
                }
                out.extend(ops);
            }
            None if matches!(o, Operand::Constant(Const::Adt(..))) => return None,
            None => out.push(o),
        }
    }
    Some(out)
}

/// An rvalue's struct value as one rvalue per leaf: a struct, or a `&mut` of one (each leaf's).
fn rvalue_ops(r: &Rvalue, fields: &Ordered<Local, Split>, shapes: &Shapes) -> Option<Vec<Rvalue>> {
    match r {
        Rvalue::Use(o) => Some(ops_of(o, fields, shapes)?.into_iter().map(Rvalue::Use).collect()),
        Rvalue::Struct(p, fs) if plain_path(p) => Some(
            built(p, fs.iter().map(|(_, o)| o.clone()), fields, shapes)?
                .into_iter()
                .map(Rvalue::Use)
                .collect(),
        ),
        Rvalue::Ref(kind @ BorrowKind::Mut, p) => match resolve(p, fields, shapes)? {
            At::Whole(ps) => Some(ps.into_iter().map(|q| Rvalue::Ref(*kind, q)).collect()),
            At::Leaf(_) => None,
        },
        _ => None,
    }
}

/// Is a whole struct (or array of structs) read as one value, or borrowed?
fn reads_whole(s: &[&Operand], borrowed: Option<&Place>, fields: &Ordered<Local, Split>, shapes: &Shapes) -> bool {
    let whole = |p: &Place| {
        p.projection.is_empty() && p.base_local().is_some_and(|l| fields.has(&l))
            || matches!(resolve(p, fields, shapes), Some(At::Whole(_)))
    };
    s.iter()
        .any(|o| matches!(o, Operand::Copy(p) | Operand::Move(p) if whole(p)))
        || borrowed.is_some_and(whole)
}

/// A statement's text without its semicolon, as messages quote it.
fn text(s: &Statement) -> String {
    s.to_string().trim_end_matches(';').to_string()
}

/// One statement of a function with struct locals, rewritten leaf by leaf (None: left as it is).
fn split_statement(
    fname: &str,
    s: &Statement,
    fields: &Ordered<Local, Split>,
    shapes: &Shapes,
) -> Result<Option<Vec<Statement>>, Fail> {
    if let Statement::Assign(d, rv) = s {
        // a whole array of structs: [S { .. }; n], [a, b, ..], or a copy of another
        if let Some(a) = d.as_local().and_then(|l| fields.get(&l)).filter(|x| x.n > 0) {
            match rv {
                Rvalue::Repeat(o, n) => {
                    let ops = ops_of(o, fields, shapes)
                        .ok_or_else(|| format!("{fname}: {}: unsupported array of structs", text(s)))?;
                    return Ok(Some(
                        ops.into_iter()
                            .enumerate()
                            .map(|(k, op)| Statement::Assign(cell(a.cells[k], &[]), Rvalue::Repeat(op, *n)))
                            .collect(),
                    ));
                }
                Rvalue::Array(items) => {
                    let els: Option<Vec<Vec<Operand>>> = items.iter().map(|e| ops_of(e, fields, shapes)).collect();
                    let els = els.ok_or_else(|| format!("{fname}: {}: unsupported array of structs", text(s)))?;
                    let column = |k: usize| Rvalue::Array(els.iter().filter_map(|e| e.get(k).cloned()).collect());
                    return Ok(Some(
                        a.cells
                            .iter()
                            .enumerate()
                            .map(|(k, c)| Statement::Assign(cell(*c, &[]), column(k)))
                            .collect(),
                    ));
                }
                Rvalue::Use(o @ (Operand::Copy(p) | Operand::Move(p))) if p.projection.is_empty() => {
                    if let Some(b) = p.base_local().and_then(|l| fields.get(&l)).filter(|x| x.n > 0) {
                        let moved = matches!(o, Operand::Move(_));
                        return Ok(Some(
                            a.cells
                                .iter()
                                .zip(&b.cells)
                                .map(|(c, from)| {
                                    let from = cell(*from, &[]);
                                    Statement::Assign(
                                        cell(*c, &[]),
                                        Rvalue::Use(if moved {
                                            Operand::Move(from)
                                        } else {
                                            Operand::Copy(from)
                                        }),
                                    )
                                })
                                .collect(),
                        ));
                    }
                }
                _ => {}
            }
            return Err(format!(
                "{fname}: {}: an array of structs used this way is not supported yet",
                text(s)
            ));
        }
        // a whole struct written: _x = S { f: a, ... }, _x = copy _y, A[i] = copy _y, (_x.k: S) = .., *_r = ..,
        // _r = &mut _x
        if let Some(At::Whole(to)) = resolve(d, fields, shapes) {
            let rvs = rvalue_ops(rv, fields, shapes)
                .filter(|rvs| rvs.len() == to.len())
                .ok_or_else(|| {
                    if d.projection.len() == 1 && indexing(&d.projection[0]) {
                        format!("{fname}: {}: unsupported element assignment", text(s))
                    } else {
                        format!("{fname}: {}: unsupported struct copy", text(s))
                    }
                })?;
            return Ok(Some(
                to.into_iter()
                    .zip(rvs)
                    .map(|(c, rv)| Statement::Assign(c, rv))
                    .collect(),
            ));
        }
    }
    let borrowed = match s {
        Statement::Assign(_, Rvalue::Ref(_, p)) => Some(p),
        _ => None,
    };
    if reads_whole(&s.operands(), borrowed, fields, shapes) {
        return Err(format!(
            "{fname}: {}: a whole struct used this way is not supported yet",
            text(s)
        ));
    }
    Ok(None)
}

/// A terminator of a function with struct locals: a call's struct arguments split, and a struct result copied
/// out of the callee's cells after it returns.
fn split_call(
    fname: &str,
    t: &Terminator,
    fields: &Ordered<Local, Split>,
    split: &HashMap<String, Ordered<Local, Split>>,
    shapes: &Shapes,
    extra: &mut Vec<Block>,
    nb: &mut u32,
) -> Result<Terminator, Fail> {
    let line = t.to_string();
    let line = line.trim_end_matches(';');
    // a call can't return an array of structs, or a struct straight into an element of one
    if let Terminator::Call { dest, target, .. } = t {
        if dest.as_local().and_then(|l| fields.get(&l)).is_some_and(|x| x.n > 0) {
            return Err(format!(
                "{fname}: {line}: an array of structs used this way is not supported yet"
            ));
        }
        if let (Some(x), [i]) = (dest.base_local(), dest.projection.as_slice()) {
            if indexing(i) && fields.get(&x).is_some_and(|x| x.n > 0) {
                return Err(format!("{fname}: {line}: unsupported element assignment"));
            }
        }
        if target.is_none() && matches!(resolve(dest, fields, shapes), Some(At::Whole(_))) {
            return Err(format!("{fname}: {line}: unsupported struct copy"));
        }
    }
    if let Terminator::Call {
        dest,
        func,
        args,
        target: Some(ret),
        unwind,
    } = t
    {
        let callee = func.to_string();
        if !callee.is_empty() && callee.chars().all(|c| c.is_ascii_alphanumeric() || "_:<>".contains(c)) {
            let flat: Vec<Operand> = args
                .iter()
                .flat_map(|a| ops_of(a, fields, shapes).unwrap_or_else(|| vec![a.clone()]))
                .collect();
            if let Some(At::Whole(to)) = resolve(dest, fields, shapes) {
                let theirs = split
                    .get(&callee)
                    .and_then(|s| s.get(&Local(0)))
                    .filter(|r| r.cells.len() == to.len())
                    .ok_or_else(|| {
                        format!("{fname}: {line}: a struct result from a function that is not compiled here")
                    })?;
                let after = BasicBlock(*nb);
                *nb += 1;
                let result = |l: Local| Place {
                    base: PlaceBase::Result {
                        func: callee.clone(),
                        local: l.0,
                    },
                    projection: vec![],
                };
                let statements = to
                    .into_iter()
                    .zip(&theirs.cells)
                    .map(|(c, from)| Statement::Assign(c, Rvalue::Use(Operand::Copy(result(*from)))))
                    .collect();
                extra.push(Block {
                    name: after,
                    statements,
                    terminator: Terminator::Goto(*ret),
                });
                return Ok(Terminator::Call {
                    dest: Place::local(Local::UNIT),
                    func: func.clone(),
                    args: flat,
                    target: Some(after),
                    unwind: unwind.clone(),
                });
            }
            return Ok(Terminator::Call {
                dest: dest.clone(),
                func: func.clone(),
                args: flat,
                target: Some(*ret),
                unwind: unwind.clone(),
            });
        }
    }
    if reads_whole(&t.operands(), None, fields, shapes) {
        return Err(format!(
            "{fname}: {line}: a whole struct used this way is not supported yet"
        ));
    }
    Ok(t.clone())
}
