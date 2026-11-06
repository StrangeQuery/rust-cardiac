//! Calls through function pointers. CARDIAC's only indirect jump is one a program builds for itself, and each
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
use std::collections::HashMap;

use super::front::{closure_name, place_ty};
use super::ir::visit_mut::VisitMut;
use super::ir::{
    occurrences, BasicBlock, Block, BorrowKind, CastKind, Const, File, IntTy, Item, Local, Operand, Path, Place,
    Projection, Rvalue, Segment, Statement, Terminator, Ty, Unsupported,
};

/// The type a function pointer is held as: a number from 0, below the count of functions of its type.
const NUMBER: Ty = Ty::Int(IntTy::U8);

/// Each function pointer a function makes, a number; each call through one, a switch to a call of each function it
/// can be; and each type that holds pointers, numbers.
pub fn function_pointers(file: &mut File) {
    let closures = closures(file);
    // the functions the program makes pointers to, by the pointer's type, in the order it first makes them
    let mut targets: Vec<(Ty, Vec<Path>)> = vec![];
    for f in file.items.iter().filter_map(|i| match i {
        Item::Fn(f) => Some(f),
        _ => None,
    }) {
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
        let types: HashMap<Local, Ty> = f
            .params
            .iter()
            .cloned()
            .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
            .collect();
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
                t => {
                    let s = t.to_string();
                    let reads_pointer = t
                        .places()
                        .iter()
                        .any(|p| place_ty(&types, p).is_none_or(|t| t.has_fn_pointer()) && touches(p, &types));
                    let passes = matches!(t, Terminator::Call { .. });
                    (reads_pointer && !passes).then(|| Terminator::Unsupported(Unsupported::of(&s)))
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
