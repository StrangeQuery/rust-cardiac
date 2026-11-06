//! Calls through function pointers and trait objects. CARDIAC's only indirect jump is one a program builds for itself, and each
//! function has cells of its own for its parameters and its result (see `lower`), so a call is made to one function,
//! known where the call is. Every function a pointer can point to is known too: rustc's MIR makes each pointer where
//! the program names the function (`double as fn(i16) -> i16 (PointerCoercion(ReifyFnPointer(Safe), Implicit))`), or a
//! closure that captures nothing (`ClosureFnPointer`). So a function pointer is a number, the function's place among
//! those the program makes pointers of that type to, and a call through one is a switch on the number to a call of
//! each of them, as a `match` on an enum calls each variant's function.
//!
//! A pointer is made, copied, kept in an array, borrowed, passed, returned and called. Anything else done with one
//! (compared, cast to an address, kept in a struct, a tuple or an enum, given to a generic function) stays as rustc
//! wrote it, and is refused where the function that does it is compiled: a number can't stand in for an address there.
//!
//! A trait object, `&dyn Trait`, is the same with a value: an enum of the types the program makes objects of
//! (`trait_objects`).
use std::collections::HashMap;

use super::enums::{EnumDef, Variant};
use super::front::{closure_name, place_ty};
use super::ir::visit_mut::{walk_ty, VisitMut};
use super::ir::{
    occurrences, parse_path, BasicBlock, Block, BorrowKind, CastKind, Const, Decl, File, FnItem, IntTy, Item, Local,
    Operand, Path, Place, Projection, Rvalue, Segment, Statement, Terminator, Ty, Unsupported,
};

/// The type a function pointer is held as: a number from 0, below the count of functions of its type.
const NUMBER: Ty = Ty::Int(IntTy::U8);

/// Each function pointer a function makes, a number; each call through one, a switch to a call of each function it
/// can be; and each type that holds pointers, numbers.
pub fn function_pointers(file: &mut File) {
    let closures = closures(file);
    // the functions the program makes pointers to, by the pointer's type, in the order it first makes them
    let mut targets: Vec<(Ty, Vec<Path>)> = vec![];
    for f in functions(file) {
        for s in f.body.blocks.iter().flat_map(|b| &b.statements) {
            let Statement::Assign(_, r) = s else { continue };
            if let Some((ty, Some(callee))) = made(r, &closures) {
                match targets.iter_mut().find(|(t, _)| *t == ty) {
                    Some((_, callees)) if callees.contains(&callee) => {}
                    Some((_, callees)) => callees.push(callee),
                    None => targets.push((ty, vec![callee])),
                }
            }
        }
    }
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let types = local_types(f);
        if !types.values().any(Ty::has_fn_pointer) && !mentions_functions(&f.body.blocks) {
            continue;
        }
        let number = |ty: &Ty, callee: &Path| -> Option<i128> {
            let (_, callees) = targets.iter().find(|(t, _)| t == ty)?;
            callees.iter().position(|c| c == callee).map(|k| k as i128)
        };
        let mut next = f.body.blocks.iter().map(|b| b.name.0).max().map_or(0, |n| n + 1);
        let mut extra = vec![];
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                let Statement::Assign(_, r) = s else {
                    continue;
                };
                // a pointer made: its function's number
                if let Some((ty, callee)) = made(r, &closures) {
                    match callee.and_then(|c| number(&ty, &c)) {
                        Some(k) => {
                            *r = Rvalue::Use(Operand::Constant(Const::Int {
                                value: k,
                                ty: IntTy::U8,
                            }))
                        }
                        None => *s = unsupported(s),
                    }
                    continue;
                }
                if mentions_pointers(s, &types) && !handled(s, &types) {
                    *s = unsupported(s);
                }
            }
            let call = match &b.terminator {
                Terminator::CallIndirect {
                    dest,
                    func,
                    args,
                    target,
                    unwind,
                } => {
                    let callees = func
                        .place()
                        .filter(|_| matches!(func, Operand::Copy(_) | Operand::Move(_)))
                        .and_then(|p| place_ty(&types, p))
                        .and_then(|ty| targets.iter().find(|(t, _)| *t == ty))
                        .map(|(_, callees)| callees.clone());
                    let call = |callee: &Path| Terminator::Call {
                        dest: dest.clone(),
                        func: callee.clone(),
                        args: args.clone(),
                        target: *target,
                        unwind: unwind.clone(),
                    };
                    match (callees.as_deref(), func.place()) {
                        // one function of the type: the call of it
                        (Some([callee]), _) => Some(call(callee)),
                        // a switch on the number: an arm for each but the last, which is the otherwise
                        (Some(callees), Some(p)) if callees.len() > 1 => {
                            let mut arms = vec![];
                            for (k, callee) in callees.iter().enumerate() {
                                let name = BasicBlock(next);
                                next += 1;
                                extra.push(Block {
                                    name,
                                    statements: vec![],
                                    terminator: call(callee),
                                });
                                arms.push((k as i128, name));
                            }
                            let otherwise = arms.pop().map(|(_, b)| b);
                            Some(Terminator::SwitchInt {
                                discr: Operand::Copy(p.clone()),
                                arms,
                                otherwise,
                            })
                        }
                        // a pointer the program never makes, or a call through anything but a place
                        _ => Some(Terminator::Unsupported(Unsupported::of(&b.terminator.to_string()))),
                    }
                }
                Terminator::Call { func, .. } if Ty::Path(func.clone()).has_fn_pointer() => {
                    Some(Terminator::Unsupported(Unsupported::of(&b.terminator.to_string())))
                }
                // a call may pass a pointer, or an array of them, and return one; nothing else reads one
                t => {
                    let call = matches!(t, Terminator::Call { .. });
                    let refused = t
                        .places()
                        .into_iter()
                        .any(|p| touches(p, &types) && !(call && whole(p, &types)));
                    refused.then(|| Terminator::Unsupported(Unsupported::of(&t.to_string())))
                }
            };
            if let Some(call) = call {
                b.terminator = call;
            }
        }
        f.body.blocks.extend(extra);
        // the types that hold pointers hold numbers
        let mut numbers = Numbers;
        for (_, t) in f.params.iter_mut() {
            numbers.visit_ty(t);
        }
        numbers.visit_ty(&mut f.ret);
        for d in f.body.decls.iter_mut() {
            numbers.visit_ty(&mut d.ty);
        }
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                if let Statement::Assign(p, r) = s {
                    numbers.visit_place(p);
                    numbers.visit_rvalue(r);
                }
            }
            numbers.visit_terminator(&mut b.terminator);
        }
    }
}

/// A function pointer's type, a pointer, an array of them, or a shared reference to either, is the number's type in
/// its place; any other type that holds one stays as it is (whatever uses it is refused).
struct Numbers;

impl VisitMut for Numbers {
    fn visit_ty(&mut self, t: &mut Ty) {
        if !t.is_fn_pointers() {
            return;
        }
        match t {
            Ty::FnPtr { .. } => *t = NUMBER,
            Ty::Array(e, _) | Ty::Ref { to: e, .. } => self.visit_ty(e),
            _ => {}
        }
    }
}

/// Each closure's type, as rustc writes it, the name its function is called by (`main_closure0`), if it can be
/// called without itself: its first parameter, the closure, never read (so the compiler drops it, as it does for a
/// closure that captures nothing).
fn closures(file: &File) -> HashMap<String, Option<String>> {
    let mut closures = HashMap::new();
    for item in &file.items {
        let Item::Fn(f) = item else { continue };
        let Some((first, t)) = f.params.first() else {
            continue;
        };
        let closure = match t {
            Ty::Ref { to, .. } => &**to,
            t => t,
        };
        let Ty::Other(closure) = closure else { continue };
        if !closure.starts_with("{closure@") {
            continue;
        }
        let unread = occurrences(&f.body.blocks, *first) == 0;
        closures.insert(closure.clone(), closure_name(&f.path).filter(|_| unread));
    }
    closures
}

/// A function pointer made: its type, and the function a call through it calls (none for a closure the compiler
/// can't call so).
fn made(r: &Rvalue, closures: &HashMap<String, Option<String>>) -> Option<(Ty, Option<Path>)> {
    match r {
        Rvalue::Cast(Operand::Fn(callee), ty @ Ty::FnPtr { .. }, CastKind::ReifyFnPointer(_)) => {
            Some((ty.clone(), Some(callee.clone())))
        }
        Rvalue::Cast(
            Operand::Constant(Const::ZeroSized(Ty::Other(closure))),
            ty @ Ty::FnPtr { .. },
            CastKind::ClosureFnPointer(_),
        ) => {
            let callee = closures.get(closure).cloned().flatten().map(|name| Path {
                qself: None,
                segments: vec![Segment::Name { name, args: None }],
            });
            Some((ty.clone(), callee))
        }
        _ => None,
    }
}

/// Whether the blocks make a pointer of a function or call through one, other than as `made` reads.
fn mentions_functions(blocks: &[Block]) -> bool {
    blocks.iter().any(|b| {
        matches!(b.terminator, Terminator::CallIndirect { .. })
            || b.statements.iter().any(|s| {
                matches!(s, Statement::Assign(_, Rvalue::Cast(Operand::Fn(_), ..)))
                    || matches!(s, Statement::Assign(_, Rvalue::Cast(_, t, _)) if t.has_fn_pointer())
            })
    })
}

/// Whether a place is of a type with a function pointer in it, or one whose type isn't known that starts from such
/// a local.
fn touches(p: &Place, types: &HashMap<Local, Ty>) -> bool {
    let local = p
        .base_local()
        .and_then(|l| types.get(&l))
        .is_some_and(Ty::has_fn_pointer);
    let field = p
        .projection
        .iter()
        .any(|x| matches!(x, Projection::Field(_, t) if t.has_fn_pointer()));
    match place_ty(types, p) {
        Some(t) => t.has_fn_pointer() || field,
        None => local || field,
    }
}

/// Whether a place is a pointer, an array of them or a reference to either, reached by no field that holds one.
fn whole(p: &Place, types: &HashMap<Local, Ty>) -> bool {
    let field = p
        .projection
        .iter()
        .any(|x| matches!(x, Projection::Field(_, t) if t.has_fn_pointer()));
    !field && place_ty(types, p).is_some_and(|t| t.is_fn_pointers())
}

/// Whether a statement reads or writes a function pointer, or makes one other than as `made` reads.
fn mentions_pointers(s: &Statement, types: &HashMap<Local, Ty>) -> bool {
    let cast =
        matches!(s, Statement::Assign(_, Rvalue::Cast(o, t, _)) if t.has_fn_pointer() || matches!(o, Operand::Fn(_)));
    cast || s.places().iter().any(|p| touches(p, types))
}

/// A statement the numbers do as the pointers would: a pointer, or an array of them, copied, built, filled or
/// borrowed.
fn handled(s: &Statement, types: &HashMap<Local, Ty>) -> bool {
    let Statement::Assign(dest, r) = s else {
        return false;
    };
    let of = |p: &Place| place_ty(types, p);
    let pointer = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => matches!(of(p), Some(Ty::FnPtr { .. })),
        _ => false,
    };
    let Some(to) = of(dest) else { return false };
    match r {
        Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => to.is_fn_pointers() && of(p).as_ref() == Some(&to),
        Rvalue::Array(os) => matches!(to, Ty::Array(..)) && to.is_fn_pointers() && os.iter().all(pointer),
        Rvalue::Repeat(o, _) => matches!(to, Ty::Array(..)) && to.is_fn_pointers() && pointer(o),
        Rvalue::Ref(BorrowKind::Shared, p) => {
            to.is_fn_pointers() && matches!(&to, Ty::Ref { to: inner, .. } if of(p).as_ref() == Some(&**inner))
        }
        _ => false,
    }
}

/// A line kept as rustc wrote it, to be refused where it is compiled.
fn unsupported(s: &Statement) -> Statement {
    Statement::Unsupported(Unsupported::of(&s.to_string()))
}

/// The types a program makes trait objects of one trait from: the enum a `&dyn Trait` is, `dyn_Shape`, and each type
/// with the name of its variant (the type's).
struct Object {
    of: Path,
    name: String,
    types: Vec<(String, Ty)>,
}

/// Trait objects. A `&dyn Trait` is a pointer to a value and a table of the functions its type implements the trait
/// with, and every type a program makes one of is known: rustc's MIR makes each where a reference to a value of the
/// type is coerced (`copy _11 as &dyn Shape (PointerCoercion(Unsize, Implicit))`). So a `&dyn Trait` is an enum of
/// those types (`dyn_Shape::Sq(Sq)`), each variant holding the value, as a shared reference to a struct is the struct
/// (nothing can change behind it while it lives); and a call of the trait's method through one is a `match` on it,
/// a call of each type's own method on its variant's value. Returns the enums, for the enum pass to lay out.
///
/// Only a shared reference to a struct of the program's, of a trait without generic arguments, whose methods are
/// the impl's own: anything else done with a trait object stays as rustc wrote it, and is refused where it is
/// compiled.
pub fn trait_objects(file: &mut File) -> Vec<EnumDef> {
    let mut objects: Vec<Object> = vec![];
    // the types each trait's objects are made of, in the order the program first makes them
    for f in functions(file) {
        let types = local_types(f);
        for s in f.body.blocks.iter().flat_map(|b| &b.statements) {
            let Some((of, ty)) = coerced(s, &types) else { continue };
            let Ty::Path(p) = &ty else { continue };
            let variant = p.name().to_string();
            match objects.iter_mut().find(|o| o.of == of) {
                Some(o) if o.types.iter().any(|(_, t)| *t == ty) => {}
                Some(o) => o.types.push((variant, ty)),
                None => objects.push(Object {
                    name: format!("dyn_{}", of.to_string().replace("::", "__")),
                    of,
                    types: vec![(variant, ty)],
                }),
            }
        }
    }
    // two types of one name would be one variant: those objects are left as they are
    objects.retain(|o| (1..o.types.len()).all(|k| o.types[..k].iter().all(|(n, _)| *n != o.types[k].0)));
    if objects.is_empty() {
        return vec![];
    }
    let paths: Vec<String> = functions(file).map(|f| f.path.to_string()).collect();
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let types = local_types(f);
        let object_of = |p: &Place| match place_ty(&types, p) {
            Some(Ty::Ref { mutable: false, to, .. }) => match &*to {
                Ty::Dyn(of) => objects.iter().find(|o| o.of == *of),
                _ => None,
            },
            _ => None,
        };
        // a place through a trait object: only a reborrow (`&(*_1)`) is the object itself
        let through = |p: &Place| {
            p.base_local()
                .and_then(|l| types.get(&l))
                .is_some_and(|t| matches!(t, Ty::Ref { to, .. } if matches!(**to, Ty::Dyn(_))))
                && !p.projection.is_empty()
        };
        let mut next_local = types
            .keys()
            .map(|l| l.0)
            .filter(|&l| l != Local::UNIT.0)
            .max()
            .map_or(0, |l| l + 1);
        let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().map_or(0, |n| n + 1);
        let (mut decls, mut extra) = (vec![], vec![]);
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                // an object made: its type's variant, holding the value
                if let Some((of, ty)) = coerced(s, &types) {
                    let o = objects.iter().find(|o| o.of == of);
                    let variant = o.and_then(|o| o.types.iter().find(|(_, t)| *t == ty).map(|(v, _)| (o, v)));
                    let (Some((o, v)), Statement::Assign(_, Rvalue::Cast(Operand::Copy(src) | Operand::Move(src), ..))) =
                        (variant, &*s)
                    else {
                        *s = unsupported(s);
                        continue;
                    };
                    let Statement::Assign(dest, _) = &*s else { continue };
                    let mut value = src.clone();
                    value.projection.push(Projection::Deref);
                    *s = Statement::Assign(
                        dest.clone(),
                        Rvalue::Variant(words(&[&o.name, v]), vec![Operand::Copy(value)]),
                    );
                    continue;
                }
                let Statement::Assign(dest, r) = &mut *s else { continue };
                // a reborrow of an object: the object
                if let Rvalue::Ref(BorrowKind::Shared, p) = r {
                    if let ([Projection::Deref], Some(l)) = (p.projection.as_slice(), p.base_local()) {
                        if object_of(&Place::local(l)).is_some() && object_of(dest).is_some() {
                            *r = Rvalue::Use(Operand::Copy(Place::local(l)));
                            continue;
                        }
                    }
                }
                let made = matches!(r, Rvalue::Cast(_, Ty::Ref { to, .. }, _) if matches!(**to, Ty::Dyn(_)));
                if made || s.places().into_iter().any(through) {
                    *s = unsupported(s);
                }
            }
            // a method called through an object: a switch on its variant, to a call of each type's method
            let dispatch = match &b.terminator {
                Terminator::Call {
                    dest,
                    func,
                    args,
                    target,
                    unwind,
                } if func.qself.as_ref().is_some_and(|q| matches!(q.ty, Ty::Dyn(_))) => {
                    let receiver = args.first().and_then(Operand::place).filter(|p| object_of(p).is_some());
                    // the trait the call names, the object's or one it extends (`<dyn Shape as Named>::name`)
                    let named = func.qself.as_ref().and_then(|q| q.as_trait.as_ref());
                    let callees: Option<Vec<(String, Ty, Path)>> =
                        receiver.and_then(object_of).zip(named).and_then(|(o, named)| {
                            o.types
                                .iter()
                                .map(|(v, ty)| Some((v.clone(), ty.clone(), method(&paths, ty, named, func.name())?)))
                                .collect()
                        });
                    match (receiver, callees) {
                        (Some(receiver), Some(callees)) => {
                            let mut arms = vec![];
                            for (v, ty, callee) in callees {
                                // the variant's value, borrowed for the method's `&self`
                                let value = Local(next_local);
                                next_local += 1;
                                decls.push(Decl {
                                    local: value,
                                    mutable: false,
                                    ty: Ty::Ref {
                                        lifetime: None,
                                        mutable: false,
                                        to: Box::new(ty.clone()),
                                    },
                                });
                                let mut field = receiver.clone();
                                field
                                    .projection
                                    .extend([Projection::Downcast(v), Projection::Field(0, ty)]);
                                let mut args = args.clone();
                                args[0] = Operand::Move(Place::local(value));
                                arms.push((
                                    Statement::Assign(Place::local(value), Rvalue::Ref(BorrowKind::Shared, field)),
                                    Terminator::Call {
                                        dest: dest.clone(),
                                        func: callee,
                                        args,
                                        target: *target,
                                        unwind: unwind.clone(),
                                    },
                                ));
                            }
                            Some((receiver.clone(), arms))
                        }
                        _ => {
                            b.terminator = Terminator::Unsupported(Unsupported::of(&b.terminator.to_string()));
                            None
                        }
                    }
                }
                t => {
                    if t.places().into_iter().any(through) {
                        b.terminator = Terminator::Unsupported(Unsupported::of(&t.to_string()));
                    }
                    None
                }
            };
            let Some((receiver, mut arms)) = dispatch else { continue };
            if arms.len() == 1 {
                let (s, t) = arms.pop().unwrap();
                b.statements.push(s);
                b.terminator = t;
                continue;
            }
            // the variant's number, as rustc reads one for a `match` (an arm for each but the last, the otherwise)
            let d = Local(next_local);
            next_local += 1;
            decls.push(Decl {
                local: d,
                mutable: true,
                ty: Ty::Int(IntTy::Isize),
            });
            b.statements
                .push(Statement::Assign(Place::local(d), Rvalue::Discriminant(receiver)));
            let mut switch = vec![];
            for (k, (s, t)) in arms.into_iter().enumerate() {
                let name = BasicBlock(next_block);
                next_block += 1;
                extra.push(Block {
                    name,
                    statements: vec![s],
                    terminator: t,
                });
                switch.push((k as i128, name));
            }
            let otherwise = switch.pop().map(|(_, b)| b);
            b.terminator = Terminator::SwitchInt {
                discr: Operand::Move(Place::local(d)),
                arms: switch,
                otherwise,
            };
        }
        f.body.decls.extend(decls);
        f.body.blocks.extend(extra);
        // a trait object's type, the enum's
        let mut enums = Objects(&objects);
        for (_, t) in f.params.iter_mut() {
            enums.visit_ty(t);
        }
        enums.visit_ty(&mut f.ret);
        enums.visit_body(&mut f.body);
    }
    objects
        .iter()
        .map(|o| EnumDef {
            path: vec![o.name.clone()],
            args: vec![],
            in_crate: true,
            variants: o
                .types
                .iter()
                .enumerate()
                .map(|(k, (v, ty))| Variant {
                    name: v.clone(),
                    value: k as i128,
                    fields: vec![ty.clone()],
                })
                .collect(),
        })
        .collect()
}

/// A `&dyn Trait` of a trait the program makes objects of: the enum's type.
struct Objects<'a>(&'a [Object]);

impl VisitMut for Objects<'_> {
    fn visit_ty(&mut self, t: &mut Ty) {
        if let Ty::Ref { mutable: false, to, .. } = t {
            if let Ty::Dyn(of) = &**to {
                if let Some(o) = self.0.iter().find(|o| o.of == *of) {
                    *t = Ty::Path(words(&[&o.name]));
                    return;
                }
            }
        }
        walk_ty(self, t)
    }
}

/// An object made, `_10 = copy _11 as &dyn Shape (PointerCoercion(Unsize, Implicit))`: the trait, and the type
/// behind the reference coerced, if it is a struct's of a plain name (and the trait's name is plain).
fn coerced(s: &Statement, types: &HashMap<Local, Ty>) -> Option<(Path, Ty)> {
    let Statement::Assign(
        _,
        Rvalue::Cast(Operand::Copy(src) | Operand::Move(src), Ty::Ref { mutable: false, to, .. }, CastKind::Unsize(_)),
    ) = s
    else {
        return None;
    };
    let Ty::Dyn(of) = &**to else { return None };
    let Some(Ty::Ref {
        mutable: false, to: ty, ..
    }) = place_ty(types, src)
    else {
        return None;
    };
    match *ty {
        Ty::Path(p) if p.is_plain() && of.is_plain() => Some((of.clone(), Ty::Path(p))),
        _ => None,
    }
}

/// The program's function that is `ty`'s method `name` of the trait `of`, as the driver names an impl's
/// (`<Sq as Shape>::area`), if there is just one: a type and a trait are known by their names, as the struct pass
/// knows a struct, and a method the impl doesn't define (the trait's default) is none.
fn method(paths: &[String], ty: &Ty, of: &Path, name: &str) -> Option<Path> {
    let Ty::Path(t) = ty else { return None };
    let found: Vec<Path> = paths
        .iter()
        .filter_map(|p| parse_path(p).ok())
        .filter(|p| {
            let Some(q) = &p.qself else { return false };
            let named = |a: &Path, b: &Path| a.is_plain() && a.name() == b.name();
            matches!(&q.ty, Ty::Path(qt) if named(qt, t))
                && q.as_trait.as_ref().is_some_and(|tr| named(tr, of))
                && matches!(p.segments.as_slice(), [Segment::Name { name: m, args: None }] if m == name)
        })
        .collect();
    match found.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// A path of plain names.
fn words(names: &[&str]) -> Path {
    Path {
        qself: None,
        segments: names
            .iter()
            .map(|n| Segment::Name {
                name: n.to_string(),
                args: None,
            })
            .collect(),
    }
}

/// The file's functions.
fn functions(file: &File) -> impl Iterator<Item = &FnItem> {
    file.items.iter().filter_map(|i| match i {
        Item::Fn(f) => Some(f),
        _ => None,
    })
}

/// A function's locals' types, its parameters' among them.
fn local_types(f: &FnItem) -> HashMap<Local, Ty> {
    f.params
        .iter()
        .cloned()
        .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::ir::parse_file;
    use super::*;

    const PROGRAM: &str = "fn inc(_1: i16) -> i16 {
    let mut _0: i16;

    bb0: {
        _0 = Add(copy _1, const 1_i16);
        return;
    }
}

fn neg(_1: i16) -> i16 {
    let mut _0: i16;

    bb0: {
        _0 = Neg(copy _1);
        return;
    }
}

fn main() -> () {
    let mut _0: ();
    let _1: fn(i16) -> i16;
    let _2: i16;
    let _3: bool;

    bb0: {
        _1 = inc as fn(i16) -> i16 (PointerCoercion(ReifyFnPointer(Safe), Implicit));
        _1 = neg as fn(i16) -> i16 (PointerCoercion(ReifyFnPointer(Safe), Implicit));
        _2 = copy _1(const 5_i16) -> [return: bb1, unwind continue];
    }

    bb1: {
        _3 = Eq(copy _1, copy _1);
        return;
    }
}
";

    fn main_of(file: &File) -> &super::super::ir::FnItem {
        file.items
            .iter()
            .find_map(|i| match i {
                Item::Fn(f) if f.path.to_string() == "main" => Some(f),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn a_pointer_is_its_function_s_number_and_a_call_through_it_a_switch() {
        let mut file = parse_file(PROGRAM);
        function_pointers(&mut file);
        let main = main_of(&file);
        let text: Vec<String> = main
            .body
            .blocks
            .iter()
            .flat_map(|b| {
                b.statements
                    .iter()
                    .map(|s| s.to_string())
                    .chain([b.terminator.to_string()])
            })
            .collect();
        assert_eq!(
            text,
            [
                "_1 = const 0_u8;",
                "_1 = const 1_u8;",
                "switchInt(copy _1) -> [0: bb2, otherwise: bb3];",
                // a comparison of two pointers is no comparison of numbers: refused where it is compiled
                "_3 = Eq(copy _1, copy _1);",
                "return;",
                "_2 = inc(const 5_i16) -> [return: bb1, unwind continue];",
                "_2 = neg(const 5_i16) -> [return: bb1, unwind continue];",
            ]
        );
        assert!(matches!(main.body.blocks[1].statements[0], Statement::Unsupported(_)));
        assert_eq!(main.body.decls.iter().find(|d| d.local == Local(1)).unwrap().ty, NUMBER);
    }
}

#[cfg(test)]
mod object_tests {
    use super::super::ir::parse_file;
    use super::*;

    const PROGRAM: &str = "fn <Sq as Shape>::area(_1: &Sq) -> i16 {
    let mut _0: i16;

    bb0: {
        _0 = copy ((*_1).0: i16);
        return;
    }
}

fn <Two as Shape>::area(_1: &Two) -> i16 {
    let mut _0: i16;

    bb0: {
        _0 = const 2_i16;
        return;
    }
}

fn show(_1: &dyn Shape) -> i16 {
    let mut _0: i16;

    bb0: {
        _0 = <dyn Shape as Shape>::area(move _1) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}

fn main() -> () {
    let mut _0: ();
    let _1: Sq;
    let _2: &Sq;
    let _3: &dyn Shape;
    let _4: &Two;

    bb0: {
        _2 = &_1;
        _3 = copy _2 as &dyn Shape (PointerCoercion(Unsize, Implicit));
        _3 = copy _4 as &dyn Shape (PointerCoercion(Unsize, Implicit));
        return;
    }
}
";

    fn lines(file: &File, name: &str) -> Vec<String> {
        let f = functions(file).find(|f| f.path.to_string() == name).unwrap();
        f.body
            .blocks
            .iter()
            .flat_map(|b| {
                b.statements
                    .iter()
                    .map(|s| s.to_string())
                    .chain([b.terminator.to_string()])
            })
            .collect()
    }

    #[test]
    fn a_trait_object_is_an_enum_of_its_types_and_a_call_through_it_a_switch() {
        let mut file = parse_file(PROGRAM);
        let enums = trait_objects(&mut file);
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].path, ["dyn_Shape"]);
        let variants: Vec<&str> = enums[0].variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(variants, ["Sq", "Two"]);
        assert_eq!(
            lines(&file, "main"),
            [
                "_2 = &_1;",
                "_3 = dyn_Shape::Sq(copy (*_2));",
                "_3 = dyn_Shape::Two(copy (*_4));",
                "return;"
            ]
        );
        assert_eq!(
            lines(&file, "show"),
            [
                "_4 = discriminant(_1);",
                "switchInt(move _4) -> [0: bb2, otherwise: bb3];",
                "return;",
                "_2 = &((_1 as Sq).0: Sq);",
                "_0 = <Sq as Shape>::area(move _2) -> [return: bb1, unwind continue];",
                "_3 = &((_1 as Two).0: Two);",
                "_0 = <Two as Shape>::area(move _3) -> [return: bb1, unwind continue];",
            ]
        );
    }
}
