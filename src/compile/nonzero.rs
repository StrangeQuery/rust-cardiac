//! `NonZero<T>` of an integer T is a T that isn't 0, and `Option<NonZero<T>>` uses 0 for `None`, as Rust lays them
//! out. rustc makes one by transmuting: `NonZero::new(x)` is `x as Option<NonZero<T>> (Transmute)`, and `get()` is
//! `n as T (Transmute)`. A `NonZero<T>` is a T here, its transmutes to and from T copies, and the transmute of x to
//! the `Option` is the test Rust's layout makes: `None` where x is 0, else `Some(x)`, the enum pass's `Option<T>`
//! (the enum's definition is normalized to it: `enums::Enums::scalar`). Any other transmute of one is left, and
//! refused.
use super::front::local_types;
use super::ir::{
    BasicBlock, BinOp, Block, CastKind, Const, File, GenericArg, Item, Local, Operand, Path, Place, Rvalue, Segment,
    Statement, Terminator, Ty,
};

/// `core::num::NonZero<T>` of an integer T: its T. With `short`, also as rustc names it where that name is
/// core's alone, `NonZero<T>` (not where the program has a type of its own of that name).
pub fn nonzero(t: &Ty, short: bool) -> Option<&Ty> {
    let Ty::Path(p) = t else { return None };
    let (last, path) = p.segments.split_last()?;
    let Segment::Name { name, args: Some(g) } = last else {
        return None;
    };
    let words = |w: &[&str]| {
        path.len() == w.len()
            && path
                .iter()
                .zip(w)
                .all(|(s, w)| matches!(s, Segment::Name { name, args: None } if name == w))
    };
    if p.qself.is_some() || name != "NonZero" || !(words(&["core", "num"]) || short && path.is_empty()) {
        return None;
    }
    match g.args.as_slice() {
        [GenericArg::Ty(t @ Ty::Int(_))] => Some(t),
        _ => None,
    }
}

/// `core::option::Option<NonZero<T>>`: the path of `Option<T>`, and T.
fn option_of_nonzero(t: &Ty, short: bool) -> Option<(Path, Ty)> {
    let Ty::Path(p) = t else { return None };
    let Some(Segment::Name { name, args: Some(g) }) = p.segments.last() else {
        return None;
    };
    let [GenericArg::Ty(inner)] = g.args.as_slice() else {
        return None;
    };
    let t = nonzero(inner, short)?.clone();
    if p.qself.is_some() || name != "Option" {
        return None;
    }
    let mut p = p.clone();
    if let Some(Segment::Name { args: Some(g), .. }) = p.segments.last_mut() {
        g.args = vec![GenericArg::Ty(t.clone())];
    }
    Some((p, t))
}

/// The type with `NonZero<T>` as T, in it anywhere a whole type argument or the type is one.
fn plain(t: &Ty, short: bool) -> Ty {
    if let Some(i) = nonzero(t, short) {
        return i.clone();
    }
    match t {
        Ty::Path(p) => {
            let mut p = p.clone();
            for s in p.segments.iter_mut() {
                if let Segment::Name { args: Some(g), .. } = s {
                    for a in g.args.iter_mut() {
                        if let GenericArg::Ty(t) = a {
                            *t = plain(t, short);
                        }
                    }
                }
            }
            Ty::Path(p)
        }
        Ty::Ref { lifetime, mutable, to } => Ty::Ref {
            lifetime: lifetime.clone(),
            mutable: *mutable,
            to: Box::new(plain(to, short)),
        },
        Ty::Array(e, n) => Ty::Array(Box::new(plain(e, short)), n.clone()),
        t => t.clone(),
    }
}

/// Every function's NonZero values as their integers, where its transmutes are the ones this file knows. `own` are
/// the names of the program's own types.
pub fn nonzero_is_its_value(file: &mut File, own: &[String]) {
    let short = !own.iter().any(|n| n == "NonZero");
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let types = local_types(&f.params, &f.body.decls);
        let mentions = |t: &Ty| plain(t, short) != *t;
        if !types.values().any(mentions) && !mentions(&f.ret) {
            continue;
        }
        // each transmute: to or from a NonZero (a copy), or to its Option (the test)
        let mut tests: Vec<(usize, usize, Place, Operand, Path, Ty)> = vec![];
        let mut copies: Vec<(usize, usize)> = vec![];
        let mut fine = true;
        for (bi, b) in f.body.blocks.iter().enumerate() {
            for (i, s) in b.statements.iter().enumerate() {
                let Statement::Assign(d, Rvalue::Cast(o, to, CastKind::Transmute)) = s else {
                    continue;
                };
                let from = o.place().and_then(|p| super::front::place_ty(&types, p));
                let copy = nonzero(to, short).is_some_and(|t| from.as_ref() == Some(t))
                    || from.as_ref().and_then(|f| nonzero(f, short)).is_some_and(|t| t == to);
                match option_of_nonzero(to, short) {
                    Some((p, t)) if from.as_ref() == Some(&t) => tests.push((bi, i, d.clone(), o.clone(), p, t)),
                    _ if copy => copies.push((bi, i)),
                    _ => fine &= plain(to, short) == *to && from.as_ref().is_none_or(|f| plain(f, short) == *f),
                }
            }
        }
        if !fine {
            continue;
        }
        for (bi, i) in copies {
            if let Statement::Assign(_, r @ Rvalue::Cast(..)) = &mut f.body.blocks[bi].statements[i] {
                let Rvalue::Cast(o, ..) = &*r else { unreachable!() };
                *r = Rvalue::Use(o.clone());
            }
        }
        let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
        let mut next_local = types.keys().map(|l| l.0).max().unwrap_or(0) + 1;
        // the tests, last first so that each block's later lines are still where they were
        for (bi, i, d, x, option, t) in tests.into_iter().rev() {
            let mut name = || {
                next_block += 1;
                BasicBlock(next_block - 1)
            };
            let (none, some, rest) = (name(), name(), name());
            let b = &mut f.body.blocks[bi];
            let after = b.statements.split_off(i + 1);
            b.statements.pop();
            let zero = Local(next_local);
            next_local += 1;
            f.body.decls.push(super::ir::Decl {
                local: zero,
                mutable: false,
                ty: Ty::Bool,
            });
            let x = match x {
                Operand::Move(p) => Operand::Copy(p),
                x => x,
            };
            let Ty::Int(it) = t else { unreachable!("an integer") };
            b.statements.push(Statement::Assign(
                Place::local(zero),
                Rvalue::BinaryOp(BinOp::Eq, x.clone(), Operand::Constant(Const::Int { value: 0, ty: it })),
            ));
            let terminator = std::mem::replace(
                &mut b.terminator,
                Terminator::SwitchInt {
                    discr: Operand::Move(Place::local(zero)),
                    arms: vec![(0, some)],
                    otherwise: Some(none),
                },
            );
            let variant = |v: &str| {
                let mut p = option.clone();
                p.segments.push(Segment::Name {
                    name: v.into(),
                    args: None,
                });
                p
            };
            f.body.blocks.push(Block {
                name: none,
                statements: vec![Statement::Assign(d.clone(), Rvalue::Variant(variant("None"), vec![]))],
                terminator: Terminator::Goto(rest),
            });
            f.body.blocks.push(Block {
                name: some,
                statements: vec![Statement::Assign(d, Rvalue::Variant(variant("Some"), vec![x]))],
                terminator: Terminator::Goto(rest),
            });
            f.body.blocks.push(Block {
                name: rest,
                statements: after,
                terminator,
            });
        }
        // and every NonZero its integer: types, and the copies
        for d in f.body.decls.iter_mut() {
            d.ty = plain(&d.ty, short);
        }
        for (_, t) in f.params.iter_mut() {
            *t = plain(t, short);
        }
        f.ret = plain(&f.ret, short);
        let mut fields = |p: &mut Place| {
            for q in p.projection.iter_mut() {
                if let super::ir::Projection::Field(_, t) = q {
                    *t = plain(t, short);
                }
            }
        };
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.places_mut(&mut fields);
            }
            b.terminator.places_mut(&mut fields);
        }
    }
}
