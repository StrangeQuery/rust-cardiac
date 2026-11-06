//! Closures that capture. A closure is a struct of what it captures, built where it is made (`{closure@src/main.rs:
//! 9:15: 9:18} { k: copy _5 }`), and a function of its own that takes that struct as its first parameter
//! (`main::{closure#0}`, which the compiler reads as `main_closure0`). Here a closure that captures is that struct, a
//! struct of its own named after its function (`main_closure0_env`), which the struct pass then takes apart as any
//! struct; and a call of it through its trait, `<{closure@..} as Fn<(i16,)>>::call(move _3, move _4)`, as rustc
//! writes one where it doesn't inline the closure (a generic function's instance calls its `F` so), is a call of its
//! function, with the arguments rustc tupled passed one by one.
//!
//! A closure that captures nothing is left as it is: it is called without itself (see `compile`).
use std::collections::HashMap;

use super::front::closure_name;
use super::ir::visit_mut::{walk_ty, VisitMut};
use super::ir::{
    occurrences, Const, File, GenericArg, Item, Local, Locals, Operand, Path, Place, Rvalue, Segment, Statement,
    Terminator, Ty, Unsupported,
};

/// A closure's function: the name the compiler reads it by, and whether it reads the closure.
struct Closure {
    function: String,
    captures: bool,
}

/// Each closure that captures as a struct, and each call of a closure through its trait as a call of its function.
/// Returns each closure that captures: its function, by the closure's type as rustc writes it (which a closure of
/// core's that wraps it still names).
pub fn closures_as_structs(file: &mut File) -> HashMap<String, String> {
    array_from_fns(file);
    let mut closures: HashMap<String, Closure> = HashMap::new();
    for item in &file.items {
        let Item::Fn(f) = item else { continue };
        let Some((first, Ty::Ref { to, .. })) = f.params.first() else {
            continue;
        };
        let (Ty::Other(closure), Some(function)) = (&**to, closure_name(&f.path)) else {
            continue;
        };
        if closure.starts_with("{closure@") {
            let captures = occurrences(&f.body.blocks, *first) > 0;
            closures.insert(closure.clone(), Closure { function, captures });
        }
    }
    // a closure built of what it captures captures, whether or not its function reads it
    for b in file.bodies().flat_map(|b| &b.blocks) {
        for s in &b.statements {
            if let Statement::Assign(_, Rvalue::Closure(Ty::Other(t), fields)) = s {
                if let Some(c) = closures.get_mut(t).filter(|_| !fields.is_empty()) {
                    c.captures = true;
                }
            }
        }
    }
    let structs: HashMap<String, String> = closures
        .iter()
        .filter(|(_, c)| c.captures)
        .map(|(t, c)| (t.clone(), format!("{}_env", c.function)))
        .collect();
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let mut named_times: HashMap<Local, usize> = HashMap::new();
        for l in f.body.blocks.iter().flat_map(|b| b.lines()).flat_map(|x| x.locals()) {
            *named_times.entry(l).or_default() += 1;
        }
        for b in f.body.blocks.iter_mut() {
            if let Some(call) = called(&b.terminator, &closures) {
                // the arguments rustc tupled, one by one: a constant's values, or what the block built the tuple of,
                // where nothing else reads it
                let Terminator::Call { args, .. } = &b.terminator else {
                    unreachable!()
                };
                let tupled = match &args[1] {
                    Operand::Constant(Const::Tuple(cs)) => {
                        Some(cs.iter().cloned().map(Operand::Constant).collect::<Vec<_>>())
                    }
                    Operand::Constant(Const::Unit) => Some(vec![]),
                    Operand::Move(p) | Operand::Copy(p) => {
                        p.as_local().filter(|t| named_times.get(t) == Some(&2)).and_then(|t| {
                            let at = b
                                .statements
                                .iter()
                                .rposition(|s| matches!(s, Statement::Assign(d, _) if d.as_local() == Some(t)))?;
                            let Statement::Assign(_, Rvalue::Tuple(os)) = &b.statements[at] else {
                                return None;
                            };
                            // (read at the call instead, what it reads must not change in between)
                            let read: Vec<Local> = os.iter().flat_map(|o| o.locals()).collect();
                            let changed = b.statements[at + 1..].iter().any(|s| match s {
                                Statement::Assign(d, _) => d.base_local().is_some_and(|l| read.contains(&l)),
                                _ => false,
                            });
                            if changed {
                                return None;
                            }
                            let os = os.clone();
                            b.statements[at] = Statement::Nop;
                            Some(os)
                        })
                    }
                    _ => None,
                };
                let Terminator::Call { args, func, .. } = &mut b.terminator else {
                    unreachable!()
                };
                match tupled {
                    Some(os) => {
                        let env = args[0].clone();
                        *args = call.captures.then_some(env).into_iter().chain(os).collect();
                        *func = Path {
                            qself: None,
                            segments: vec![Segment::Name {
                                name: call.function.clone(),
                                args: None,
                            }],
                        };
                    }
                    None => b.terminator = Terminator::Unsupported(Unsupported::of(&b.terminator.to_string())),
                }
            }
            for s in b.statements.iter_mut() {
                let Statement::Assign(_, r) = s else { continue };
                match r {
                    Rvalue::Closure(Ty::Other(t), fields) if structs.contains_key(t) => {
                        *r = Rvalue::Struct(named(&structs[t]), std::mem::take(fields));
                    }
                    // (core's in `Option::get_or_insert`, which returns its capture, is `enum_methods`')
                    Rvalue::Closure(Ty::Other(t), _) if t.ends_with("::get_or_insert::{closure#0}}") => {}
                    // a closure of core's, which wraps the program's (`map_fold(f, g)`): the adaptor pass reads it,
                    // and refuses what it doesn't take
                    Rvalue::Closure(Ty::Other(t), _) if !closures.contains_key(t) => {}
                    // a closure of another kind, or a tuple of one that no call took apart, which the passes don't
                    // know
                    Rvalue::Closure(..) => *s = Statement::Unsupported(Unsupported::of(&s.to_string())),
                    Rvalue::Tuple(os) if os.len() == 1 => *s = Statement::Unsupported(Unsupported::of(&s.to_string())),
                    _ => {}
                }
            }
        }
        let mut types = Structs(&structs);
        for (_, t) in f.params.iter_mut() {
            types.visit_ty(t);
        }
        types.visit_ty(&mut f.ret);
        types.visit_body(&mut f.body);
    }
    closures
        .into_iter()
        .filter(|(_, c)| c.captures)
        .map(|(t, c)| (t, c.function))
        .collect()
}

/// The closure a call calls through its trait, `<{closure@..} as Fn<(i16,)>>::call(env, args)` (or `FnMut`'s
/// `call_mut`), if it is one whose function the compiler reads.
fn called<'a>(t: &Terminator, closures: &'a HashMap<String, Closure>) -> Option<&'a Closure> {
    let Terminator::Call { func, args, .. } = t else {
        return None;
    };
    let q = func.qself.as_ref()?;
    let Ty::Other(closure) = &q.ty else { return None };
    let tr = q.as_trait.as_ref()?;
    let method = match tr.segments.as_slice() {
        [Segment::Name { name, args: Some(g) }]
            if matches!(g.args.as_slice(), [GenericArg::Ty(Ty::Tuple(_) | Ty::Unit)]) =>
        {
            name.as_str()
        }
        _ => return None,
    };
    let called = matches!(
        (method, func.segments.as_slice()),
        ("Fn", [Segment::Name { name, args: None }]) if name == "call"
    ) || matches!(
        (method, func.segments.as_slice()),
        ("FnMut", [Segment::Name { name, args: None }]) if name == "call_mut"
    );
    (called && args.len() == 2).then(|| closures.get(closure)).flatten()
}

/// A path of one name.
fn named(name: &str) -> Path {
    Path {
        qself: None,
        segments: vec![Segment::Name {
            name: name.to_string(),
            args: None,
        }],
    }
}

/// A closure that captures, by its type: its struct.
struct Structs<'a>(&'a HashMap<String, String>);

impl VisitMut for Structs<'_> {
    fn visit_ty(&mut self, t: &mut Ty) {
        if let Ty::Other(text) = t {
            if let Some(s) = self.0.get(text) {
                *t = Ty::Path(named(s));
                return;
            }
        }
        walk_ty(self, t)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ir::parse_file;
    use super::*;

    const PROGRAM: &str = "fn main::{closure#0}(_1: &{closure@src/main.rs:9:15: 9:18}, _2: i16) -> i16 {
    let mut _0: i16;
    let mut _3: i16;

    bb0: {
        _3 = copy ((*_1).0: i16);
        _0 = Add(copy _2, move _3);
        return;
    }
}

fn main() -> () {
    let mut _0: ();
    let _1: i16;
    let _2: {closure@src/main.rs:9:15: 9:18};
    let mut _3: &{closure@src/main.rs:9:15: 9:18};
    let mut _4: (i16,);
    let _5: i16;

    bb0: {
        _2 = {closure@src/main.rs:9:15: 9:18} { k: copy _1 };
        _3 = &_2;
        _4 = (const 4_i16,);
        _5 = <{closure@src/main.rs:9:15: 9:18} as Fn<(i16,)>>::call(move _3, move _4) -> [return: bb1, unwind continue];
    }

    bb1: {
        return;
    }
}
";

    #[test]
    fn a_closure_that_captures_is_a_struct_and_its_call_a_call_of_its_function() {
        let mut file = parse_file(PROGRAM);
        closures_as_structs(&mut file);
        let Some(Item::Fn(main)) = file
            .items
            .iter()
            .find(|i| matches!(i, Item::Fn(f) if f.path.to_string() == "main"))
        else {
            panic!("no main")
        };
        let b = &main.body.blocks[0];
        assert_eq!(b.statements[0].to_string(), "_2 = main_closure0_env { k: copy _1 };");
        assert_eq!(b.statements[2], Statement::Nop);
        assert_eq!(
            b.terminator.to_string(),
            "_5 = main_closure0(move _3, const 4_i16) -> [return: bb1, unwind continue];"
        );
        let env = main.body.decls.iter().find(|d| d.local == Local(2)).unwrap();
        assert_eq!(env.ty, Ty::Path(named("main_closure0_env")));
    }
}

/// `core::array::from_fn(f)`, which rustc inlines into `try_from_fn` given core's closure that wraps f
/// (`NeverShortCircuit::wrap_mut_1`), its result a `NeverShortCircuit<[T; N]>` read as its field `.0`: the loop
/// core's makes, `t[i] = f(i)` for i from 0 up to N, f called through `FnMut` as rustc calls a closure, which the
/// pass above then makes a call of its function. The result is the array itself.
fn array_from_fns(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let mut next_local = f
            .params
            .iter()
            .map(|(l, _)| l.0)
            .chain(f.body.decls.iter().map(|d| d.local.0))
            .max()
            .unwrap_or(0)
            + 1;
        let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
        let mut extra = vec![];
        for bi in 0..f.body.blocks.len() {
            let Terminator::Call {
                dest,
                func,
                args,
                target: Some(ret),
                unwind,
            } = &f.body.blocks[bi].terminator
            else {
                continue;
            };
            let Some(Segment::Name { name, args: Some(g) }) = func.segments.last() else {
                continue;
            };
            let [GenericArg::Ty(Ty::Path(never)), GenericArg::Const(n), GenericArg::Ty(Ty::Other(wrap))] =
                g.args.as_slice()
            else {
                continue;
            };
            let wraps = wrap.starts_with("{closure@ops::try_trait::NeverShortCircuit<")
                && wrap.contains(">::wrap_mut_1<usize, ");
            if name != "try_from_fn" || never.name() != "NeverShortCircuit" || !wraps {
                continue;
            }
            let ([Operand::Move(w)], Some(result)) = (args.as_slice(), dest.as_local()) else {
                continue;
            };
            let Some(w) = w.as_local() else { continue };
            // the wrapper, built in this block of f alone
            let Some(at) = f.body.blocks[bi].statements.iter().position(|s| matches!(s, Statement::Assign(d, Rvalue::Closure(_, fs)) if d.as_local() == Some(w) && fs.len() == 1)) else {
                continue;
            };
            let Statement::Assign(_, Rvalue::Closure(_, fs)) = &f.body.blocks[bi].statements[at] else {
                unreachable!()
            };
            let Some(user) = fs[0].1.local() else { continue };
            let types = super::front::local_types(&f.params, &f.body.decls);
            let (Some(Ty::Other(closure)), Some(Ty::Path(nsc))) = (types.get(&user), types.get(&result)) else {
                continue;
            };
            let Some(GenericArg::Ty(array @ Ty::Array(element, _))) = nsc.segments.last().and_then(|s| match s {
                Segment::Name { args: Some(g), .. } => g.args.first(),
                _ => None,
            }) else {
                continue;
            };
            // the result read only as its field, `(_10.0: [i16; 4])`
            let field = |p: &Place| {
                p.base_local() == Some(result)
                    && matches!(p.projection.as_slice(), [super::ir::Projection::Field(0, _)])
            };
            let mut reads = 0;
            for b in &f.body.blocks {
                for s in &b.statements {
                    s.places().iter().filter(|p| field(p)).for_each(|_| reads += 1);
                }
            }
            let named: usize = f
                .body
                .blocks
                .iter()
                .flat_map(|b| b.lines())
                .map(|x| x.count(result))
                .sum();
            let wrapper_named: usize = f.body.blocks.iter().flat_map(|b| b.lines()).map(|x| x.count(w)).sum();
            let storage = f
                .body
                .blocks
                .iter()
                .flat_map(|b| &b.statements)
                .filter(
                    |s| matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if *l == result || *l == w),
                )
                .count();
            if named + wrapper_named != reads + 1 + 2 + storage {
                continue;
            }
            let (closure, element, array, ret, unwind, n) = (
                closure.clone(),
                (**element).clone(),
                array.clone(),
                *ret,
                unwind.clone(),
                *n,
            );
            let mut local = |ty: Ty| {
                next_local += 1;
                f.body.decls.push(super::ir::Decl {
                    local: Local(next_local - 1),
                    mutable: true,
                    ty,
                });
                Local(next_local - 1)
            };
            let usize_ty = Ty::Int(super::ir::IntTy::Usize);
            let (i, c, r, t, v) = (
                local(usize_ty.clone()),
                local(Ty::Bool),
                local(Ty::Ref {
                    lifetime: None,
                    mutable: true,
                    to: Box::new(Ty::Other(closure.clone())),
                }),
                local(Ty::Tuple(vec![usize_ty.clone()])),
                local(element),
            );
            for d in f.body.decls.iter_mut().filter(|d| d.local == result) {
                d.ty = array.clone();
            }
            let mut name = || {
                next_block += 1;
                super::ir::BasicBlock(next_block - 1)
            };
            let (head, body, step) = (name(), name(), name());
            let int = |k: u64| {
                Operand::Constant(Const::Int {
                    value: k as i128,
                    ty: super::ir::IntTy::Usize,
                })
            };
            let copy = |l: Local| Operand::Copy(super::ir::Place::local(l));
            let assign = |l: Local, rv: Rvalue| Statement::Assign(super::ir::Place::local(l), rv);
            let fn_mut = Path {
                qself: Some(Box::new(super::ir::QSelf {
                    ty: Ty::Other(closure),
                    as_trait: Some(Path {
                        qself: None,
                        segments: vec![Segment::Name {
                            name: "FnMut".into(),
                            args: Some(super::ir::GenericArgs {
                                turbofish: false,
                                args: vec![GenericArg::Ty(Ty::Tuple(vec![usize_ty]))],
                            }),
                        }],
                    }),
                })),
                segments: vec![Segment::Name {
                    name: "call_mut".into(),
                    args: None,
                }],
            };
            extra.push(super::ir::Block {
                name: head,
                statements: vec![assign(c, Rvalue::BinaryOp(super::ir::BinOp::Lt, copy(i), int(n)))],
                terminator: Terminator::SwitchInt {
                    discr: Operand::Move(super::ir::Place::local(c)),
                    arms: vec![(0, ret)],
                    otherwise: Some(body),
                },
            });
            extra.push(super::ir::Block {
                name: body,
                statements: vec![
                    assign(
                        r,
                        Rvalue::Ref(super::ir::BorrowKind::Mut, super::ir::Place::local(user)),
                    ),
                    assign(t, Rvalue::Tuple(vec![copy(i)])),
                ],
                terminator: Terminator::Call {
                    dest: super::ir::Place::local(v),
                    func: fn_mut,
                    args: vec![
                        Operand::Move(super::ir::Place::local(r)),
                        Operand::Move(super::ir::Place::local(t)),
                    ],
                    target: Some(step),
                    unwind,
                },
            });
            let element_at = Place {
                base: super::ir::PlaceBase::Local(result),
                projection: vec![super::ir::Projection::Index(i)],
            };
            extra.push(super::ir::Block {
                name: step,
                statements: vec![
                    Statement::Assign(element_at, Rvalue::Use(Operand::Move(super::ir::Place::local(v)))),
                    assign(i, Rvalue::BinaryOp(super::ir::BinOp::Add, copy(i), int(1))),
                ],
                terminator: Terminator::Goto(head),
            });
            let b = &mut f.body.blocks[bi];
            b.statements[at] = assign(i, Rvalue::Use(int(0)));
            b.terminator = Terminator::Goto(head);
            // the result's field is the result
            for b in f.body.blocks.iter_mut() {
                for s in b.statements.iter_mut() {
                    s.places_mut(&mut |p| {
                        if field(p) {
                            p.projection.clear();
                        }
                    });
                }
            }
        }
        f.body.blocks.extend(extra);
    }
}
