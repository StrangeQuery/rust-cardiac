//! The methods of `Option` and `Result` where rustc leaves them as calls: `x.unwrap_or(0)`, `x.is_some()`, the `?`
//! operator's `Try::branch` and `FromResidual::from_residual`. rustc's inliner, which the driver runs from opt-level
//! 1 up (`driver/src/inline.rs`), inlines most of them into a test of the discriminant and the variants' fields;
//! where it doesn't (a body past its threshold, or a build with `-Zinline-mir=no`) each call becomes that here, as
//! blocks of the caller's own, before the enum pass (`enums::data_enums`) lays the enums out in cells. A variant's
//! number comes from the enum's definition, as the driver gave it.
//!
//! ```text
//!     _5 = Option::<i16>::unwrap_or(move _6, const -5_i16) -> [return: bb4, unwind continue];
//! ```
//!
//! is
//!
//! ```text
//!     _9 = discriminant(_6);
//!     switchInt(move _9) -> [1: bb10, otherwise: bb11];
//! bb10: _5 = move ((_6 as Some).0: i16); goto -> bb4;
//! bb11: _5 = const -5_i16; goto -> bb4;
//! ```
//!
//! `unwrap()` and `expect()` on the failing variant call what core's do, `core::option::unwrap_failed` and its
//! kin, which never return and say Rust's message. `map` with a closure that captures nothing calls the
//! closure's own function (`main::{closure#0}`, read as `main_closure0`) on the good variant's value, and builds the
//! good variant of what it returns; a closure that captures stays a call, and is refused. `map_or`, `map_or_else`,
//! `or_else` and `filter`, which rustc calls even at opt-level 2 (their bodies are past its inliner's threshold),
//! call their closures the same way.
use std::collections::HashMap;

use super::enums::{variant_number, EnumDef};
use super::ir::{
    BasicBlock, BinOp, Block, BorrowKind, Const, Decl, GenericArg, IntTy, Local, Operand, Path, Place, Projection,
    Rvalue, Segment, Statement, Terminator, Ty,
};
use super::structs::Function;

/// The type arguments of a named type (`T` and `E` of `Result<T, E>`).
fn ty_args(t: &Ty) -> Vec<Ty> {
    let Ty::Path(p) = t else { return vec![] };
    p.segments
        .iter()
        .flat_map(|s| match s {
            Segment::Name { args: Some(a), .. } => a.args.clone(),
            _ => vec![],
        })
        .filter_map(|a| match a {
            GenericArg::Ty(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// The last word of a named type's path: `Option` for `core::option::Option<i16>`.
fn ty_name(t: &Ty) -> Option<&str> {
    let Ty::Path(p) = t else { return None };
    match p.segments.last()? {
        Segment::Name { name, .. } => Some(name),
        _ => None,
    }
}

/// The path of a variant of an enum type: `core::option::Option<i16>`'s `Some`.
fn variant_path(t: &Ty, v: &str) -> Option<Path> {
    let Ty::Path(p) = t else { return None };
    let mut p = p.clone();
    p.segments.push(Segment::Name {
        name: v.into(),
        args: None,
    });
    Some(p)
}

/// A variant's field, `((x as V).0: T)`.
fn field(x: &Place, v: &str, t: &Ty) -> Place {
    let mut p = x.clone();
    p.projection.push(Projection::Downcast(v.into()));
    p.projection.push(Projection::Field(0, t.clone()));
    p
}

/// The method a call is to, and whether it is the `?` operator's (`Try::branch`, `FromResidual::from_residual`):
/// `Option::<i16>::unwrap_or`, `core::result::Result::<i16, i16>::ok`, `<Option<i16> as Try>::branch`.
fn called(func: &Path) -> Option<&str> {
    // (`ok_or::<i16>` names its own type argument)
    let Some(Segment::Name { name, .. }) = func.segments.last() else {
        return None;
    };
    match &func.qself {
        None => {
            let [.., Segment::Name { name: of, .. }, _] = func.segments.as_slice() else {
                return None;
            };
            (of == "Option" || of == "Result").then_some(name.as_str())
        }
        Some(q) => {
            let t = q.as_trait.as_ref()?;
            let of = ty_name(&q.ty)?;
            let ok = (of == "Option" || of == "Result")
                && func.segments.len() == 1
                && matches!(
                    (t.name(), name.as_str()),
                    ("Try", "branch") | ("FromResidual", "from_residual")
                );
            ok.then_some(name.as_str())
        }
    }
}

/// What a call becomes: statements at the end of its block, then either a jump on to where the call returned to or
/// a test of a discriminant into two new blocks, each with its statements (and, for a failing `unwrap()`, the
/// panic in place of the jump).
struct Inlined {
    before: Vec<Statement>,
    /// the tested local, the variant number that goes to the first way, and the two ways
    test: Option<(Local, i128, Way, Way)>,
}

/// One way out of a test: its statements, then a jump on, or the panic call (its path and arguments), or a call
/// (its path, arguments and result) after which `after` runs and the way goes on, or with `then`, the call's
/// result (a bool) is tested and one set of statements or the other runs.
struct Way {
    statements: Vec<Statement>,
    panic: Option<(Path, Vec<Operand>)>,
    call: Option<(Path, Vec<Operand>, Local, Vec<Statement>)>,
    then: Option<(Vec<Statement>, Vec<Statement>)>,
}

fn way(statements: Vec<Statement>) -> Way {
    Way {
        statements,
        panic: None,
        call: None,
        then: None,
    }
}

fn core_path(words: &[&str]) -> Path {
    Path {
        qself: None,
        segments: words
            .iter()
            .map(|w| Segment::Name {
                name: w.to_string(),
                args: None,
            })
            .collect(),
    }
}

/// Each call to a method of `Option` or `Result` this file knows, in one function, as the blocks rustc would have
/// inlined. `defs` are the enums' definitions as `enums::normalized` gives them; `closures`, each closure's
/// function by the closure's type (as rustc writes it).
pub fn inline_enum_methods(f: &mut Function, defs: &[EnumDef], closures: &HashMap<String, String>) {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    types.insert(Local(0), f.ret.clone());
    let ty_of = |p: &Place| -> Option<Ty> {
        let mut t = types.get(&p.base_local()?)?.clone();
        for x in &p.projection {
            t = match (x, t) {
                (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
                (Projection::Field(_, t), _) => t.clone(),
                _ => return None,
            };
        }
        Some(t)
    };
    let next_local = std::cell::Cell::new(
        types
            .keys()
            .map(|l| l.0)
            .filter(|&l| l != Local::UNIT.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let mut decls = vec![];
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
        let Some(method) = called(func) else { continue };
        let ret = *ret;
        let unwind = unwind.clone();
        let place = |o: &Operand| match o {
            Operand::Copy(p) | Operand::Move(p) => Some(p.clone()),
            _ => None,
        };
        // the enum the method is of, and the place of the value it takes apart (behind its reference, for is_some)
        let by_ref = matches!(method, "is_some" | "is_none" | "is_ok" | "is_err");
        let (enum_ty, x) = if method == "from_residual" {
            (ty_of(dest), args.first().and_then(place))
        } else {
            let Some(mut x) = args.first().and_then(place) else {
                continue;
            };
            if by_ref {
                x.projection.push(Projection::Deref);
            }
            (ty_of(&x), Some(x))
        };
        let Some(enum_ty) = enum_ty else { continue };
        let option = match ty_name(&enum_ty) {
            Some("Option") => true,
            Some("Result") => false,
            _ => continue,
        };
        let (good, bad) = if option { ("Some", "None") } else { ("Ok", "Err") };
        let (Some(good_n), Some(bad_n)) = (
            variant_number(defs, &enum_ty, good),
            variant_number(defs, &enum_ty, bad),
        ) else {
            continue;
        };
        let targs = ty_args(&enum_ty);
        let (Some(t), e) = (targs.first().cloned(), targs.get(1).cloned()) else {
            continue;
        };
        let dest_ty = ty_of(dest);
        let fresh = |ty: Ty, decls: &mut Vec<Decl>| {
            let l = Local(next_local.get());
            next_local.set(l.0 + 1);
            decls.push(Decl {
                local: l,
                mutable: true,
                ty,
            });
            l
        };
        let set = |rv: Rvalue| Statement::Assign(dest.clone(), rv);
        let build = |ty: &Option<Ty>, v: &str, ops: Vec<Operand>| -> Option<Statement> {
            Some(set(Rvalue::Variant(variant_path(ty.as_ref()?, v)?, ops)))
        };
        let moved = |p: Place| Operand::Move(p);
        let discr = |decls: &mut Vec<Decl>| -> Option<(Local, Statement)> {
            let x = x.clone()?;
            let d = fresh(Ty::Int(IntTy::I16), decls);
            Some((d, Statement::Assign(Place::local(d), Rvalue::Discriminant(x))))
        };
        let mut new_decls = vec![];
        let inlined: Option<Inlined> = (|| {
            let x = x.clone();
            Some(match method {
                "is_some" | "is_none" | "is_ok" | "is_err" => {
                    let (d, s) = discr(&mut new_decls)?;
                    let n = if matches!(method, "is_some" | "is_ok") {
                        good_n
                    } else {
                        bad_n
                    };
                    Inlined {
                        before: vec![
                            s,
                            set(Rvalue::BinaryOp(
                                BinOp::Eq,
                                Operand::Copy(Place::local(d)),
                                Operand::Constant(Const::Int {
                                    value: n,
                                    ty: IntTy::I16,
                                }),
                            )),
                        ],
                        test: None,
                    }
                }
                "unwrap_or" | "unwrap" | "expect" | "ok_or" | "ok" | "err" | "branch" => {
                    let x = x?;
                    let (d, s) = discr(&mut new_decls)?;
                    let good_v = moved(field(&x, good, &t));
                    let (first, n, second) = match method {
                        "unwrap_or" => (
                            way(vec![set(Rvalue::Use(good_v))]),
                            good_n,
                            way(vec![set(Rvalue::Use(args.get(1)?.clone()))]),
                        ),
                        "unwrap" | "expect" => {
                            let panic = match (option, method) {
                                (true, "unwrap") => (core_path(&["core", "option", "unwrap_failed"]), vec![]),
                                (true, _) => (
                                    core_path(&["core", "option", "expect_failed"]),
                                    vec![args.get(1)?.clone()],
                                ),
                                (false, m) => {
                                    let msg = match m {
                                        "unwrap" => Operand::Constant(Const::Str(
                                            "called `Result::unwrap()` on an `Err` value".into(),
                                        )),
                                        _ => args.get(1)?.clone(),
                                    };
                                    (core_path(&["core", "result", "unwrap_failed"]), vec![msg])
                                }
                            };
                            (
                                way(vec![set(Rvalue::Use(good_v))]),
                                good_n,
                                Way {
                                    statements: vec![],
                                    panic: Some(panic),
                                    call: None,
                                    then: None,
                                },
                            )
                        }
                        // Option -> Result
                        "ok_or" if option => (
                            way(vec![build(&dest_ty, "Ok", vec![good_v])?]),
                            good_n,
                            way(vec![build(&dest_ty, "Err", vec![args.get(1)?.clone()])?]),
                        ),
                        // Result -> Option
                        "ok" if !option => (
                            way(vec![build(&dest_ty, "Some", vec![good_v])?]),
                            good_n,
                            way(vec![build(&dest_ty, "None", vec![])?]),
                        ),
                        "err" if !option => (
                            way(vec![build(&dest_ty, "Some", vec![moved(field(&x, bad, e.as_ref()?))])?]),
                            bad_n,
                            way(vec![build(&dest_ty, "None", vec![])?]),
                        ),
                        // `?`: the value goes on (Continue), else the residual, the failing variant of
                        // Option<Infallible> or Result<Infallible, E>, breaks out (Break)
                        "branch" => {
                            let residual_ty = ty_args(dest_ty.as_ref()?).first()?.clone();
                            let r = fresh(residual_ty.clone(), &mut new_decls);
                            let residual = if option {
                                vec![]
                            } else {
                                vec![moved(field(&x, bad, e.as_ref()?))]
                            };
                            (
                                way(vec![build(&dest_ty, "Continue", vec![good_v])?]),
                                good_n,
                                way(vec![
                                    Statement::Assign(
                                        Place::local(r),
                                        Rvalue::Variant(variant_path(&residual_ty, bad)?, residual),
                                    ),
                                    build(&dest_ty, "Break", vec![moved(Place::local(r))])?,
                                ]),
                            )
                        }
                        _ => return None,
                    };
                    Inlined {
                        before: vec![s],
                        test: Some((d, n, first, second)),
                    }
                }
                // map(f), f a closure that captures nothing: on the good variant, its value through the closure's
                // function, as the good variant of the result; the other variant as it is (None, or Err of its E)
                "map" => {
                    let x = x?;
                    let [_, Operand::Constant(Const::ZeroSized(_))] = args.as_slice() else {
                        return None;
                    };
                    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
                        return None;
                    };
                    let [GenericArg::Ty(u), GenericArg::Ty(Ty::Other(closure))] = g.args.as_slice() else {
                        return None;
                    };
                    let callee = closures.get(closure)?;
                    let (d, s) = discr(&mut new_decls)?;
                    let r = fresh(u.clone(), &mut new_decls);
                    let other = if option {
                        build(&dest_ty, "None", vec![])?
                    } else {
                        build(&dest_ty, "Err", vec![moved(field(&x, bad, e.as_ref()?))])?
                    };
                    let good_way = Way {
                        statements: vec![],
                        panic: None,
                        call: Some((
                            core_path(&[callee]),
                            vec![moved(field(&x, good, &t))],
                            r,
                            vec![build(&dest_ty, good, vec![moved(Place::local(r))])?],
                        )),
                        then: None,
                    };
                    Inlined {
                        before: vec![s],
                        test: Some((d, good_n, good_way, way(vec![other]))),
                    }
                }
                // filter(p), p a closure that captures nothing: on Some, p on a reference to its value, and Some
                // of the value where it holds, else None; None as it is
                "filter" if option => {
                    let x = x?;
                    let [_, Operand::Constant(Const::ZeroSized(_))] = args.as_slice() else {
                        return None;
                    };
                    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
                        return None;
                    };
                    let [GenericArg::Ty(Ty::Other(closure))] = g.args.as_slice() else {
                        return None;
                    };
                    let callee = closures.get(closure)?;
                    let value = field(&x, good, &t);
                    let to = fresh(
                        Ty::Ref {
                            lifetime: None,
                            mutable: false,
                            to: Box::new(t.clone()),
                        },
                        &mut new_decls,
                    );
                    let holds = fresh(Ty::Bool, &mut new_decls);
                    let none = || build(&dest_ty, bad, vec![]);
                    let good_way = Way {
                        statements: vec![Statement::Assign(
                            Place::local(to),
                            Rvalue::Ref(BorrowKind::Shared, value.clone()),
                        )],
                        panic: None,
                        call: Some((core_path(&[callee]), vec![moved(Place::local(to))], holds, vec![])),
                        then: Some((vec![build(&dest_ty, good, vec![moved(value)])?], vec![none()?])),
                    };
                    let (d, s) = discr(&mut new_decls)?;
                    Inlined {
                        before: vec![s],
                        test: Some((d, good_n, good_way, way(vec![none()?]))),
                    }
                }
                // map_or(default, f), map_or_else(d, f) and or_else(f), each closure one that captures nothing: on
                // the good variant, map_or's and map_or_else's value through f, or_else's variant as it is; on the
                // other, map_or's default, or the closure d (f for or_else) on nothing (Option) or the error (Result)
                "map_or" | "map_or_else" | "or_else" => {
                    let x = x?;
                    let closures_given = if method == "map_or_else" { 2 } else { 1 };
                    let (value, given) = args.split_at(args.len() - closures_given);
                    if value.len() != if method == "map_or" { 2 } else { 1 }
                        || !given
                            .iter()
                            .all(|a| matches!(a, Operand::Constant(Const::ZeroSized(_))))
                    {
                        return None;
                    }
                    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
                        return None;
                    };
                    // the closures' types, the method's last type arguments, d's before f's
                    let tys: Vec<&Ty> = g
                        .args
                        .iter()
                        .filter_map(|a| match a {
                            GenericArg::Ty(t) => Some(t),
                            _ => None,
                        })
                        .collect();
                    let (d, f) = (*tys.get(tys.len().checked_sub(closures_given)?)?, *tys.last()?);
                    let result = dest_ty.clone()?;
                    let mut through = |closure: &Ty, args: Vec<Operand>| -> Option<Way> {
                        let Ty::Other(closure) = closure else { return None };
                        let callee = closures.get(closure)?;
                        let r = fresh(result.clone(), &mut new_decls);
                        Some(Way {
                            statements: vec![],
                            panic: None,
                            call: Some((
                                core_path(&[callee]),
                                args,
                                r,
                                vec![set(Rvalue::Use(moved(Place::local(r))))],
                            )),
                            then: None,
                        })
                    };
                    let good_v = moved(field(&x, good, &t));
                    let on_bad = if option {
                        vec![]
                    } else {
                        vec![moved(field(&x, bad, e.as_ref()?))]
                    };
                    let (good_way, bad_way) = match method {
                        "map_or" => (through(f, vec![good_v])?, way(vec![set(Rvalue::Use(value[1].clone()))])),
                        "map_or_else" => (through(f, vec![good_v])?, through(d, on_bad)?),
                        _ => (way(vec![build(&dest_ty, good, vec![good_v])?]), through(f, on_bad)?),
                    };
                    let (d, s) = discr(&mut new_decls)?;
                    Inlined {
                        before: vec![s],
                        test: Some((d, good_n, good_way, bad_way)),
                    }
                }
                // the residual as the function's own failing variant: None, or Err with the residual's E. That is
                // `From::from` of it only where the two E are the same type (core's `impl From<T> for T`); another
                // E's conversion is a call this can't make, so the call stays, and is refused
                "from_residual" => {
                    let built = if option {
                        build(&dest_ty, "None", vec![])?
                    } else {
                        let r = x?;
                        let residual_e = ty_args(&ty_of(&r)?).get(1)?.clone();
                        if Some(&residual_e) != e.as_ref() {
                            return None;
                        }
                        build(&dest_ty, "Err", vec![moved(field(&r, "Err", &residual_e))])?
                    };
                    Inlined {
                        before: vec![built],
                        test: None,
                    }
                }
                _ => return None,
            })
        })();
        let Some(inlined) = inlined else { continue };
        decls.extend(new_decls);
        let b = &mut f.body.blocks[bi];
        b.statements.extend(inlined.before);
        let Some((d, n, first, second)) = inlined.test else {
            b.terminator = Terminator::Goto(ret);
            continue;
        };
        let mut way_block = |w: Way, decls: &mut Vec<Decl>| {
            let name = BasicBlock(next_block);
            next_block += 1;
            let terminator = match (w.panic, w.call) {
                (None, Some((func, args, r, after))) => {
                    let back = BasicBlock(next_block);
                    next_block += 1;
                    let terminator = match w.then {
                        None => Terminator::Goto(ret),
                        Some((yes, no)) => {
                            let mut on = |statements: Vec<Statement>| {
                                let name = BasicBlock(next_block);
                                next_block += 1;
                                extra.push(Block {
                                    name,
                                    statements,
                                    terminator: Terminator::Goto(ret),
                                });
                                name
                            };
                            let (yes, no) = (on(yes), on(no));
                            Terminator::SwitchInt {
                                discr: Operand::Move(Place::local(r)),
                                arms: vec![(0, no)],
                                otherwise: Some(yes),
                            }
                        }
                    };
                    extra.push(Block {
                        name: back,
                        statements: after,
                        terminator,
                    });
                    Terminator::Call {
                        dest: Place::local(r),
                        func,
                        args,
                        target: Some(back),
                        unwind: unwind.clone(),
                    }
                }
                (None, None) => Terminator::Goto(ret),
                (Some((func, args)), _) => {
                    let unit = Local(next_local.get());
                    next_local.set(unit.0 + 1);
                    decls.push(Decl {
                        local: unit,
                        mutable: false,
                        ty: Ty::Unit,
                    });
                    Terminator::Call {
                        dest: Place::local(unit),
                        func,
                        args,
                        target: None,
                        unwind: unwind.clone(),
                    }
                }
            };
            extra.push(Block {
                name,
                statements: w.statements,
                terminator,
            });
            name
        };
        let one = way_block(first, &mut decls);
        let two = way_block(second, &mut decls);
        f.body.blocks[bi].terminator = Terminator::SwitchInt {
            discr: Operand::Move(Place::local(d)),
            arms: vec![(n, one)],
            otherwise: Some(two),
        };
    }
    f.body.blocks.extend(extra);
    f.body.decls.extend(decls);
}

/// `a.checked_add(b)`, `checked_sub` and `checked_mul` on an i16, which rustc calls at opt-level 1 and s
/// (`core::num::<impl i16>::checked_add`): what it inlines at 2, the overflow-checked op and a test of its flag,
/// `Some` of the value or `None`. On CARDIAC the op overflows past a card (999), as every i16 op does.
pub fn inline_checked(f: &mut Function) {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    types.insert(Local(0), f.ret.clone());
    let mut next_local = types
        .keys()
        .map(|l| l.0)
        .filter(|&l| l != Local::UNIT.0)
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
            ..
        } = &f.body.blocks[bi].terminator
        else {
            continue;
        };
        let op = match (&func.qself, func.segments.as_slice()) {
            (
                None,
                [Segment::Name { name: c, .. }, Segment::Name { name: n, .. }, Segment::Impl(Ty::Int(IntTy::I16)), Segment::Name { name: m, args: None }],
            ) if (c == "core" || c == "std") && n == "num" => match m.as_str() {
                "checked_add" => BinOp::AddWithOverflow,
                "checked_sub" => BinOp::SubWithOverflow,
                "checked_mul" => BinOp::MulWithOverflow,
                _ => continue,
            },
            _ => continue,
        };
        let ([a, b], Some(option)) = (args.as_slice(), dest.as_local().and_then(|d| types.get(&d))) else {
            continue;
        };
        let (Some(some), Some(none)) = (variant_path(option, "Some"), variant_path(option, "None")) else {
            continue;
        };
        let (dest, ret, a, b) = (dest.clone(), *ret, a.clone(), b.clone());
        let pair = Local(next_local);
        let flag = Local(next_local + 1);
        next_local += 2;
        f.body.decls.push(Decl {
            local: pair,
            mutable: true,
            ty: Ty::Tuple(vec![Ty::Int(IntTy::I16), Ty::Bool]),
        });
        f.body.decls.push(Decl {
            local: flag,
            mutable: true,
            ty: Ty::Bool,
        });
        let (yes, no) = (BasicBlock(next_block), BasicBlock(next_block + 1));
        next_block += 2;
        let part = |k: u32, t: Ty| Place::field(pair, k, t);
        extra.push(Block {
            name: yes,
            statements: vec![Statement::Assign(
                dest.clone(),
                Rvalue::Variant(some, vec![Operand::Copy(part(0, Ty::Int(IntTy::I16)))]),
            )],
            terminator: Terminator::Goto(ret),
        });
        extra.push(Block {
            name: no,
            statements: vec![Statement::Assign(dest, Rvalue::Variant(none, vec![]))],
            terminator: Terminator::Goto(ret),
        });
        let block = &mut f.body.blocks[bi];
        block
            .statements
            .push(Statement::Assign(Place::local(pair), Rvalue::BinaryOp(op, a, b)));
        block.statements.push(Statement::Assign(
            Place::local(flag),
            Rvalue::Use(Operand::Copy(part(1, Ty::Bool))),
        ));
        block.terminator = Terminator::SwitchInt {
            discr: Operand::Move(Place::local(flag)),
            arms: vec![(0, yes)],
            otherwise: Some(no),
        };
    }
    f.body.blocks.extend(extra);
}

/// `a == b` and `a != b` of two `Option`s of a scalar, which rustc calls at opt-level 1 and s
/// (`<Option<i16> as PartialEq>::eq(&a, &b)`): the same variant, and, for `Some`, the same payload. The borrows go
/// with the enum pass's `forward_refs`, the places they borrow read as they are.
pub fn inline_option_eq(f: &mut Function, defs: &[EnumDef]) {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    types.insert(Local(0), f.ret.clone());
    let mut next_local = types
        .keys()
        .map(|l| l.0)
        .filter(|&l| l != Local::UNIT.0)
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
            ..
        } = &f.body.blocks[bi].terminator
        else {
            continue;
        };
        let Some(q) = &func.qself else { continue };
        let negated = match func.segments.as_slice() {
            [Segment::Name { name, args: None }] if name == "eq" => false,
            [Segment::Name { name, args: None }] if name == "ne" => true,
            _ => continue,
        };
        if !q.as_trait.as_ref().is_some_and(|t| t.name() == "PartialEq") || ty_name(&q.ty) != Some("Option") {
            continue;
        }
        let payload = ty_args(&q.ty).first().cloned();
        let Some(payload @ (Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool)) = payload else {
            continue;
        };
        let Some(some_n) = variant_number(defs, &q.ty, "Some") else {
            continue;
        };
        let [Operand::Copy(a) | Operand::Move(a), Operand::Copy(b) | Operand::Move(b)] = args.as_slice() else {
            continue;
        };
        let deref = |p: &Place| {
            let mut p = p.clone();
            p.projection.push(Projection::Deref);
            p
        };
        let (a, b, dest, ret) = (deref(a), deref(b), dest.clone(), *ret);
        let mut local = |ty: Ty, f: &mut Function| {
            let l = Local(next_local);
            next_local += 1;
            f.body.decls.push(Decl {
                local: l,
                mutable: true,
                ty,
            });
            l
        };
        // each discriminant read only to be switched on, so that with a niche or a sign tag each is a TAC
        let (ta, tb, tb2) = (
            local(Ty::Int(IntTy::I16), f),
            local(Ty::Int(IntTy::I16), f),
            local(Ty::Int(IntTy::I16), f),
        );
        let mut block = |statements: Vec<Statement>, terminator: Terminator| {
            let name = BasicBlock(next_block);
            next_block += 1;
            extra.push(Block {
                name,
                statements,
                terminator,
            });
            name
        };
        let answer = |v: Rvalue| Statement::Assign(dest.clone(), v);
        let truth = |t: bool| Rvalue::Use(Operand::Constant(Const::Bool(t != negated)));
        let unequal = block(vec![answer(truth(false))], Terminator::Goto(ret));
        let both_none = block(vec![answer(truth(true))], Terminator::Goto(ret));
        let op = if negated { BinOp::Ne } else { BinOp::Eq };
        let payloads = block(
            vec![answer(Rvalue::BinaryOp(
                op,
                Operand::Copy(field(&a, "Some", &payload)),
                Operand::Copy(field(&b, "Some", &payload)),
            ))],
            Terminator::Goto(ret),
        );
        // switch on t: Some to one way, None the other
        let on = |t: Local, some: BasicBlock, none: BasicBlock| Terminator::SwitchInt {
            discr: Operand::Move(Place::local(t)),
            arms: vec![(some_n, some)],
            otherwise: Some(none),
        };
        let read = |t: Local, x: &Place| Statement::Assign(Place::local(t), Rvalue::Discriminant(x.clone()));
        let a_some = block(vec![read(tb, &b)], on(tb, payloads, unequal));
        let a_none = block(vec![read(tb2, &b)], on(tb2, unequal, both_none));
        let bk = &mut f.body.blocks[bi];
        bk.statements.push(read(ta, &a));
        bk.terminator = on(ta, a_some, a_none);
    }
    f.body.blocks.extend(extra);
}
