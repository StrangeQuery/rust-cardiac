//! `Ordering`, and rustc's three-way compare. An `Ordering` is a C-like enum (`Less`, `Equal`, `Greater`: -1, 0
//! and 1), so once the C-like pass has run it is one cell holding its number. What makes one is `Cmp(a, b)`, as
//! rustc inlines `a.cmp(&b)` at opt-level 2 (and `partial_cmp`, `signum` and the derived comparisons), or at
//! opt-level 1 the call `<i16 as Ord>::cmp(&a, &b)`; what reads one at opt-level 1 is a call to one of its methods,
//! `o.is_lt()` or `a.then(b)`, which rustc inlines at 2. Each becomes tests here, before the C-like pass, written
//! with the enum's own variants, which the C-like pass then makes their numbers:
//!   d = Cmp(a, b)          if a < b { Less } else if a > b { Greater } else { Equal }
//!   d = o.is_lt()          d = o < Equal (and so for is_le, is_gt, is_ge, is_eq, is_ne)
//!   d = a.then(b)          if a == Equal { b } else { a }
//!   d = o.reverse()        d = Cmp(Equal, o)
//! `Cmp` is two tests, not a subtraction and its sign: a - b may be past a card.
use super::front::{local_types, place_ty};
use super::ir::visit_mut::{walk_path, VisitMut};
use super::ir::{
    BinOp, Block, Body, Const, File, GenericArg, GenericArgs, Item, Local, Operand, Path, Place, Projection, Rvalue,
    Segment, Statement, Terminator, Ty, Unwind,
};
use super::methods::Gains;

/// A call this file makes tests of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    /// `<T as Ord>::cmp`, for an integer type T
    IntCmp,
    /// `Ordering::is_lt` and its kin: the comparison with `Equal`
    Is(BinOp),
    /// `Ordering::then`
    Then,
    /// `Ordering::reverse`
    Reverse,
}

/// The call a callee's path is: `<i16 as Ord>::cmp`, `core::cmp::Ordering::is_lt`.
fn call(func: &Path) -> Option<Call> {
    if let Some(q) = &func.qself {
        let ord = q.as_trait.as_ref().is_some_and(|t| t.is_path(&["Ord"]));
        let cmp = matches!(func.segments.as_slice(), [Segment::Name { name, args: None }] if name == "cmp");
        return (ord && cmp && matches!(q.ty, Ty::Int(_))).then_some(Call::IntCmp);
    }
    let [c, cmp, ordering, Segment::Name { name, args: None }] = func.segments.as_slice() else {
        return None;
    };
    let std = |s: &Segment| matches!(s, Segment::Name { name, args: None } if name == "core" || name == "std");
    let is = |s: &Segment, n: &str| matches!(s, Segment::Name { name, args: None } if name == n);
    if !(func.is_plain() && std(c) && is(cmp, "cmp") && is(ordering, "Ordering")) {
        return None;
    }
    Some(match name.as_str() {
        "is_lt" => Call::Is(BinOp::Lt),
        "is_le" => Call::Is(BinOp::Le),
        "is_gt" => Call::Is(BinOp::Gt),
        "is_ge" => Call::Is(BinOp::Ge),
        "is_eq" => Call::Is(BinOp::Eq),
        "is_ne" => Call::Is(BinOp::Ne),
        "then" => Call::Then,
        "reverse" => Call::Reverse,
        _ => return None,
    })
}

/// Is a type `core::cmp::Ordering`, as the MIR names it?
fn is_ordering(p: &Path) -> bool {
    p.is_path(&["core", "cmp", "Ordering"]) || p.is_path(&["std", "cmp", "Ordering"])
}

/// `const core::cmp::Ordering::Less`: a variant of the enum, by the path its type has.
fn variant(ty: &Path, name: &str) -> Operand {
    let mut p = ty.clone();
    p.segments.push(Segment::Name {
        name: name.into(),
        args: None,
    });
    Operand::Constant(Const::Path(p))
}

/// An operand read more than once: a copy, not a move.
fn copied(o: &Operand) -> Operand {
    match o {
        Operand::Move(p) => Operand::Copy(p.clone()),
        o => o.clone(),
    }
}

/// The first `d = Cmp(a, b)` in the blocks: its block's index and its own.
fn first_cmp(blocks: &[Block]) -> Option<(usize, usize)> {
    blocks.iter().enumerate().find_map(|(at, b)| {
        let i = b
            .statements
            .iter()
            .position(|s| matches!(s, Statement::Assign(_, Rvalue::BinaryOp(BinOp::Cmp, ..))))?;
        Some((at, i))
    })
}

/// Every `Cmp`, `<T as Ord>::cmp` on an integer, and call to an `Ordering` method in the file becomes its tests.
pub fn three_way_compares(file: &mut File) {
    for item in &mut file.items {
        if let Item::Fn(f) = item {
            compares_in(&f.params, &mut f.body);
        }
    }
}

/// A function's, given its parameters and body.
fn compares_in(params: &[(Local, Ty)], body: &mut Body) {
    // the calls first: `<i16 as Ord>::cmp` becomes a `Cmp`, which the loop below makes tests
    let types = local_types(params, &body.decls);
    let ordering = |o: &Operand| match o.place().and_then(|p| place_ty(&types, p)) {
        Some(Ty::Path(p)) if is_ordering(&p) => Some(p),
        _ => None,
    };
    let mut gains = Gains::new(params, &body.blocks, &mut body.decls);
    for block in body.blocks.iter_mut() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(back),
            unwind: Unwind::Continue | Unwind::Unreachable,
        } = &block.terminator
        else {
            continue;
        };
        let (dest, back) = (dest.clone(), *back);
        let assign = |r: Rvalue| Statement::Assign(dest.clone(), r);
        let args: Vec<Operand> = args.iter().map(copied).collect();
        match (call(func), args.as_slice()) {
            // `&a` and `&b`: what they borrow is compared
            (Some(Call::IntCmp), [Operand::Copy(a), Operand::Copy(b)]) => {
                let deref = |p: &Place| {
                    let mut p = p.clone();
                    p.projection.push(Projection::Deref);
                    Operand::Copy(p)
                };
                let cmp = assign(Rvalue::BinaryOp(BinOp::Cmp, deref(a), deref(b)));
                block.statements.push(cmp);
                block.terminator = Terminator::Goto(back);
            }
            (Some(Call::Is(op)), [o]) => {
                let Some(ty) = ordering(o) else { continue };
                let is = assign(Rvalue::BinaryOp(op, o.clone(), variant(&ty, "Equal")));
                block.statements.push(is);
                block.terminator = Terminator::Goto(back);
            }
            (Some(Call::Reverse), [o]) => {
                let Some(ty) = ordering(o) else { continue };
                let cmp = assign(Rvalue::BinaryOp(BinOp::Cmp, variant(&ty, "Equal"), o.clone()));
                block.statements.push(cmp);
                block.terminator = Terminator::Goto(back);
            }
            (Some(Call::Then), [first, second]) => {
                let Some(ty) = ordering(first) else { continue };
                let take_second = gains.set(&dest, Rvalue::Use(second.clone()), back);
                let keep_first = gains.set(&dest, Rvalue::Use(first.clone()), back);
                let (s, t) = gains.test(BinOp::Eq, first, &variant(&ty, "Equal"), take_second, keep_first);
                block.statements.push(s);
                block.terminator = t;
            }
            _ => {}
        }
    }
    let gained = std::mem::take(&mut gains.blocks);
    body.blocks.extend(gained);
    // then each `Cmp`, its block split in two: the statements after it go on in a block of their own
    while let Some((at, i)) = first_cmp(&body.blocks) {
        let types = local_types(params, &body.decls);
        let Statement::Assign(dest, Rvalue::BinaryOp(_, a, b)) = body.blocks[at].statements[i].clone() else {
            unreachable!("first_cmp found a Cmp")
        };
        // not an `Ordering` the C-like pass will number: left for the lowering to refuse
        let Some(Ty::Path(ty)) = place_ty(&types, &dest).filter(|t| matches!(t, Ty::Path(p) if is_ordering(p))) else {
            break;
        };
        let (a, b) = (copied(&a), copied(&b));
        let mut gains = Gains::new(params, &body.blocks, &mut body.decls);
        let block = &mut body.blocks[at];
        let after = block.statements.split_off(i + 1);
        block.statements.pop();
        let end = std::mem::replace(&mut block.terminator, Terminator::Unreachable);
        let rest = gains.block(after, end);
        let mut set = |v: &str| gains.set(&dest, Rvalue::Use(variant(&ty, v)), rest);
        let (less, greater, equal) = (set("Less"), set("Greater"), set("Equal"));
        let (s, t) = gains.test(BinOp::Gt, &a, &b, greater, equal);
        let above = gains.block(vec![s], t);
        let (s, t) = gains.test(BinOp::Lt, &a, &b, less, above);
        let gained = std::mem::take(&mut gains.blocks);
        let block = &mut body.blocks[at];
        block.statements.push(s);
        block.terminator = t;
        body.blocks.extend(gained);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir;

    fn lines(body: &Body) -> Vec<String> {
        let blocks = body.blocks.iter();
        blocks
            .flat_map(|b| {
                let head = format!("bb{}:", b.name.0);
                let statements = b.statements.iter().map(|s| s.to_string());
                std::iter::once(head)
                    .chain(statements)
                    .chain([b.terminator.to_string()])
            })
            .collect()
    }

    #[test]
    fn a_call_to_cmp_becomes_two_tests_of_what_its_references_borrow() {
        let mut body = ir::parse_body(
            "let mut _0: ();\n let _1: i16;\n let _2: i16;\n let mut _3: &i16;\n let mut _4: &i16;\n let mut _5: core::cmp::Ordering;\n bb0: {\n _3 = &_1;\n _4 = &_2;\n _5 = <i16 as Ord>::cmp(move _3, move _4) -> [return: bb1, unwind continue];\n }\n bb1: {\n return;\n }\n",
        );
        compares_in(&[], &mut body);
        assert_eq!(
            lines(&body),
            [
                "bb0:",
                "_3 = &_1;",
                "_4 = &_2;",
                "_7 = Lt(copy (*_3), copy (*_4));",
                "switchInt(move _7) -> [0: bb6, otherwise: bb3];",
                "bb1:",
                "return;",
                "bb2:",
                "goto -> bb1;",
                "bb3:",
                "_5 = const core::cmp::Ordering::Less;",
                "goto -> bb2;",
                "bb4:",
                "_5 = const core::cmp::Ordering::Greater;",
                "goto -> bb2;",
                "bb5:",
                "_5 = const core::cmp::Ordering::Equal;",
                "goto -> bb2;",
                "bb6:",
                "_6 = Gt(copy (*_3), copy (*_4));",
                "switchInt(move _6) -> [0: bb5, otherwise: bb4];",
            ]
        );
    }
}

/// `core::cmp::Reverse<T>` of an integer T: its T.
fn reversed(t: &Ty) -> Option<Ty> {
    let Ty::Path(p) = t else { return None };
    let core = p.segments.len() == 3
        && p.segments[..2]
            .iter()
            .zip(["core", "cmp"])
            .all(|(s, w)| matches!(s, Segment::Name { name, args: None } if name == w));
    if p.qself.is_some() || !(core || p.segments.len() == 1) {
        return None;
    }
    let Some(Segment::Name { name, args: Some(g) }) = p.segments.last() else {
        return None;
    };
    match g.args.as_slice() {
        [GenericArg::Ty(t @ Ty::Int(_))] if name == "Reverse" => Some(t.clone()),
        _ => None,
    }
}

/// Does a type, a place's or an operand's, name `Reverse` anywhere?
struct NamesReverse(bool);

impl VisitMut for NamesReverse {
    fn visit_path(&mut self, p: &mut Path) {
        self.0 |= p
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Name { name, .. } if name == "Reverse"));
        walk_path(self, p)
    }
}

/// `Reverse(x)` of an integer is x, compared the other way round: a `Reverse<T>` local is a T, `Reverse(x)` is x and
/// its `.0` the value, and a comparison of two (`<Reverse<T> as PartialOrd>::lt(&a, &b)`, `cmp`) is T's of the two
/// swapped; their `max` is T's `min`, and `min` T's `max` (equal values are the same value). In a function whose
/// parameters and result have none, and where nothing else is done with one; else it is left, and refused.
pub fn reverse_is_its_value(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let mut names = NamesReverse(false);
        for d in f.body.decls.iter_mut() {
            names.visit_ty(&mut d.ty);
        }
        let mut outside = NamesReverse(false);
        for (_, t) in f.params.iter_mut() {
            outside.visit_ty(t);
        }
        outside.visit_ty(&mut f.ret);
        if !names.0 || outside.0 {
            continue;
        }
        let types = local_types(&f.params, &f.body.decls);
        let mut body = f.body.clone();
        // each `(p.0: T)` of a Reverse p, p
        let unwrap = |p: &mut Place| {
            let mut t = types.get(&p.base_local().unwrap_or(Local(u32::MAX))).cloned();
            let mut k = 0;
            while k < p.projection.len() {
                if let (Some(_), Projection::Field(0, _)) = (t.as_ref().and_then(reversed), &p.projection[k]) {
                    t = t.as_ref().and_then(reversed);
                    p.projection.remove(k);
                    continue;
                }
                t = t.and_then(|t| match (&p.projection[k], t) {
                    (Projection::Deref, Ty::Ref { to, .. }) => Some(*to),
                    (Projection::Field(_, ft), _) => Some(ft.clone()),
                    _ => None,
                });
                k += 1;
            }
        };
        for b in body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.places_mut(&mut |p| unwrap(p));
                if let Statement::Assign(_, r @ Rvalue::Variant(..)) = s {
                    let Rvalue::Variant(p, ops) = &*r else { unreachable!() };
                    if let (Some(_), [o]) = (
                        reversed(&Ty::Path(Path {
                            qself: None,
                            segments: p.segments.clone(),
                        })),
                        ops.as_slice(),
                    ) {
                        *r = Rvalue::Use(o.clone());
                    }
                }
            }
            b.terminator.places_mut(&mut |p| unwrap(p));
            if let Terminator::Call { func, args, .. } = &mut b.terminator {
                if let Some(swapped) = swapped_call(func) {
                    *func = swapped.0;
                    if swapped.1 {
                        args.reverse();
                    }
                }
            }
        }
        for d in body.decls.iter_mut() {
            d.ty = match &d.ty {
                Ty::Ref { lifetime, mutable, to } if reversed(to).is_some() => Ty::Ref {
                    lifetime: lifetime.clone(),
                    mutable: *mutable,
                    to: Box::new(reversed(to).unwrap()),
                },
                t => reversed(t).unwrap_or_else(|| t.clone()),
            };
        }
        let mut left = NamesReverse(false);
        left.visit_body(&mut body.clone());
        if !left.0 {
            f.body = body;
        }
    }
}

/// A call of a comparison of two `Reverse<T>`s as T's: its path, and whether the arguments swap.
fn swapped_call(func: &Path) -> Option<(Path, bool)> {
    let one = |name: &str, args: Option<GenericArgs>| Segment::Name {
        name: name.into(),
        args,
    };
    if let Some(q) = &func.qself {
        let t = reversed(&q.ty)?;
        let tr = q.as_trait.as_ref()?;
        let [Segment::Name { name, args: None }] = func.segments.as_slice() else {
            return None;
        };
        let (name, swap) = match (tr.name(), name.as_str()) {
            ("Ord", "max") => ("min", false),
            ("Ord", "min") => ("max", false),
            ("Ord", "cmp") => ("cmp", true),
            ("PartialOrd", n @ ("lt" | "le" | "gt" | "ge" | "partial_cmp")) => (n, true),
            _ => return None,
        };
        let mut q = (**q).clone();
        q.ty = t;
        return Some((
            Path {
                qself: Some(Box::new(q)),
                segments: vec![one(name, None)],
            },
            swap,
        ));
    }
    // core::cmp::max::<Reverse<T>>
    let [c, cmp, Segment::Name { name, args: Some(g) }] = func.segments.as_slice() else {
        return None;
    };
    let words = matches!(c, Segment::Name { name, args: None } if name == "core")
        && matches!(cmp, Segment::Name { name, args: None } if name == "cmp");
    let [GenericArg::Ty(r)] = g.args.as_slice() else {
        return None;
    };
    let t = reversed(r)?;
    let name = match name.as_str() {
        "max" if words => "min",
        "min" if words => "max",
        _ => return None,
    };
    let args = GenericArgs {
        turbofish: g.turbofish,
        args: vec![GenericArg::Ty(t)],
    };
    Some((
        Path {
            qself: None,
            segments: vec![c.clone(), cmp.clone(), one(name, Some(args))],
        },
        false,
    ))
}
