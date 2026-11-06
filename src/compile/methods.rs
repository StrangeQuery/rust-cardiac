//! The small integer methods of the standard library, where rustc leaves them as calls: `x.abs()`, `a.min(b)`,
//! `x.rem_euclid(7)`. At opt-level 2 and 3 rustc inlines most of them into a test or two and some arithmetic; at
//! opt-level 1 and s it calls them, across the crate boundary, and there is no MIR here to compile for the callee.
//! (`min`, `max` and `clamp` it calls at every level.) Each call becomes here what rustc would have inlined: the
//! same tests and arithmetic, as blocks of the caller's own, so every pass after sees them as it sees the program's
//! own code (the range pass knows `x.abs()` is 0 or more, say, and a check on it can go). What rustc inlines at
//! opt-level 2 into something refused stays refused there: `signum` becomes a three-way compare (`Cmp`), and a
//! u8's `abs_diff` a subtraction in i32.
//!
//! What a card can't hold needs no code: no card holds `i16::MIN`, so `abs` and `%` have no overflow to check, and
//! none of these methods depends on the build's overflow checks. A check that is a panic in Rust whatever the
//! profile, `rem_euclid`'s divisor of zero and `clamp`'s `min <= max`, stays.
use super::ir::{
    BasicBlock, BinOp, Block, BorrowKind, Const, Decl, GenericArg, IntTy, Local, Operand, Path, Place, Projection,
    Rvalue, Segment, Statement, Terminator, Ty, UnOp, Unwind,
};
use super::structs::Function;

/// A method this file knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Abs,
    Signum,
    IsNegative,
    IsPositive,
    WrappingAdd,
    WrappingSub,
    WrappingMul,
    WrappingNeg,
    RemEuclid,
    DivEuclid,
    Min,
    Max,
    Clamp,
    AbsDiff,
    SaturatingSub,
}

/// The types it compiles them for: the card's own number for the signed methods, and every integer type for the
/// ones that only compare (`min`, `max`, `clamp`) or can't leave the type's range (`abs_diff`, `saturating_sub` of
/// an unsigned type). The rest, on u8 or usize, wrap or saturate where a card does not (at 256, at 2^64).
fn method(name: &str, ty: IntTy) -> Option<Method> {
    use Method::*;
    let signed = ty == IntTy::I16;
    let unsigned = matches!(ty, IntTy::U8 | IntTy::Usize);
    Some(match name {
        "abs" | "wrapping_abs" if signed => Abs,
        "signum" if signed => Signum,
        "is_negative" if signed => IsNegative,
        "is_positive" if signed => IsPositive,
        "wrapping_add" if signed => WrappingAdd,
        "wrapping_sub" if signed => WrappingSub,
        "wrapping_mul" if signed => WrappingMul,
        "wrapping_neg" if signed => WrappingNeg,
        "rem_euclid" if signed => RemEuclid,
        "div_euclid" if signed => DivEuclid,
        "min" if signed || unsigned => Min,
        "max" if signed || unsigned => Max,
        "clamp" if signed || unsigned => Clamp,
        "abs_diff" if unsigned => AbsDiff,
        "saturating_sub" if unsigned => SaturatingSub,
        _ => return None,
    })
}

/// Whether the method `name` of the integer type `ty` is one this file writes where rustc calls it. At opt-level 1
/// and s the driver keeps rustc's inliner off these (`driver/src/inline.rs`), so they are calls written here as
/// before, and not what rustc would inline (for `signum` and a u8's `abs_diff`, something refused).
pub fn written_here(name: &str, ty: IntTy) -> bool {
    method(name, ty).is_some()
}

/// The method a call is to, and the integer type it is on, from the callee's path: `core::num::<impl i16>::abs`,
/// `<u8 as Ord>::min`, `core::cmp::max::<usize>`, or `std::intrinsics::saturating_sub::<usize>`, which is what
/// `saturating_sub` is at opt-level 2.
fn called(func: &Path) -> Option<(Method, IntTy)> {
    let int = |t: &Ty| match t {
        Ty::Int(t) => Some(*t),
        _ => None,
    };
    let named =
        |s: &Segment, names: &[&str]| matches!(s, Segment::Name { name, args: None } if names.contains(&name.as_str()));
    let std = |s: &Segment| named(s, &["core", "std"]);
    // `name::<T>`
    fn generic(s: &Segment) -> Option<(&str, IntTy)> {
        match s {
            Segment::Name { name, args: Some(a) } => match a.args.as_slice() {
                [GenericArg::Ty(Ty::Int(t))] => Some((name.as_str(), *t)),
                _ => None,
            },
            _ => None,
        }
    }
    let ord = |m: &Method| matches!(m, Method::Min | Method::Max | Method::Clamp);
    let (m, ty) = match (&func.qself, func.segments.as_slice()) {
        // <T as Ord>::min
        (Some(q), [Segment::Name { name: m, args: None }])
            if q.as_trait.as_ref().is_some_and(|t| t.is_plain() && t.name() == "Ord") =>
        {
            let t = int(&q.ty)?;
            (method(m, t).filter(ord)?, t)
        }
        (None, [c, n, Segment::Impl(t), Segment::Name { name: m, args: None }]) if std(c) && named(n, &["num"]) => {
            let t = int(t)?;
            (method(m, t).filter(|m| !ord(m))?, t)
        }
        (None, [c, cmp, f]) if std(c) && named(cmp, &["cmp"]) => {
            let (m, t) = generic(f)?;
            (method(m, t).filter(|m| matches!(m, Method::Min | Method::Max))?, t)
        }
        (None, [c, i, f]) if std(c) && named(i, &["intrinsics"]) => {
            let (m, t) = generic(f)?;
            (method(m, t).filter(|m| *m == Method::SaturatingSub)?, t)
        }
        _ => return None,
    };
    Some((m, ty))
}

/// `(a..b).contains(&x)` or `(a..=b).contains(&x)`, as rustc calls it at opt-level 1 and s:
/// `core::ops::RangeInclusive::<i16>::contains::<i16>`, or through the trait,
/// `<core::ops::RangeInclusive<i16> as RangeBounds<i16>>::contains::<i16>`. Inclusive or not, and the type.
fn contains_call(func: &Path) -> Option<(bool, IntTy)> {
    let range = |name: &str, args: &Option<super::ir::GenericArgs>| {
        let incl = match name {
            "Range" => false,
            "RangeInclusive" => true,
            _ => return None,
        };
        match args.as_ref()?.args.as_slice() {
            [GenericArg::Ty(Ty::Int(t))] => Some((incl, *t)),
            _ => None,
        }
    };
    let core_ops = |c: &Segment, o: &Segment| {
        matches!((c, o), (Segment::Name { name: c, args: None }, Segment::Name { name: o, args: None })
            if (c == "core" || c == "std") && o == "ops")
    };
    let contains = |s: &Segment| matches!(s, Segment::Name { name, .. } if name == "contains");
    match (&func.qself, func.segments.as_slice()) {
        (None, [c, o, Segment::Name { name, args }, m]) if core_ops(c, o) && contains(m) => range(name, args),
        (Some(q), [m]) if contains(m) && q.as_trait.as_ref().is_some_and(|t| t.name() == "RangeBounds") => {
            match &q.ty {
                Ty::Path(p) => match p.segments.as_slice() {
                    [c, o, Segment::Name { name, args }] if core_ops(c, o) => range(name, args),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// The blocks and locals a function gains as its calls to the methods become code.
struct Gains<'a> {
    blocks: Vec<Block>,
    decls: &'a mut Vec<Decl>,
    next_local: u32,
    next_block: u32,
}

impl Gains<'_> {
    fn local(&mut self, ty: Ty) -> Local {
        let l = Local(self.next_local);
        self.next_local += 1;
        self.decls.push(Decl {
            local: l,
            mutable: true,
            ty,
        });
        l
    }

    fn block(&mut self, statements: Vec<Statement>, terminator: Terminator) -> BasicBlock {
        let name = BasicBlock(self.next_block);
        self.next_block += 1;
        self.blocks.push(Block {
            name,
            statements,
            terminator,
        });
        name
    }

    /// `t = Op(a, b)` and a switch on it: the statement for the block that ends in the switch.
    fn test(
        &mut self,
        op: BinOp,
        a: &Operand,
        b: &Operand,
        yes: BasicBlock,
        no: BasicBlock,
    ) -> (Statement, Terminator) {
        let t = self.local(Ty::Bool);
        (
            Statement::Assign(Place::local(t), Rvalue::BinaryOp(op, a.clone(), b.clone())),
            Terminator::SwitchInt {
                discr: Operand::Move(Place::local(t)),
                arms: vec![(0, no)],
                otherwise: Some(yes),
            },
        )
    }

    /// A block that sets the call's destination and goes back to where the call went on to.
    fn set(&mut self, dest: &Place, v: Rvalue, back: BasicBlock) -> BasicBlock {
        self.block(vec![Statement::Assign(dest.clone(), v)], Terminator::Goto(back))
    }

    /// The code of `dest = m(args)`, in place of the call: the statements for the call's own block, and its new
    /// terminator.
    fn expand(
        &mut self,
        m: Method,
        ty: IntTy,
        dest: &Place,
        args: &[Operand],
        back: BasicBlock,
    ) -> Option<(Vec<Statement>, Terminator)> {
        use Rvalue::{BinaryOp, Use};
        let zero = Operand::Constant(Const::Int { value: 0, ty });
        let int = |v: i128| Operand::Constant(Const::Int { value: v, ty });
        let assign = |v: Rvalue| Statement::Assign(dest.clone(), v);
        let go = Terminator::Goto(back);
        let copy = |o: &Operand| Use(o.clone());
        Some(match (m, args) {
            // if x < 0 { -x } else { x }: on a card, never i16::MIN
            (Method::Abs, [x]) => {
                let neg = self.set(dest, Rvalue::UnaryOp(UnOp::Neg, x.clone()), back);
                let keep = self.set(dest, copy(x), back);
                let (s, t) = self.test(BinOp::Lt, x, &zero, neg, keep);
                (vec![s], t)
            }
            // -1 below 0, 1 above it, 0 at it
            (Method::Signum, [x]) => {
                let minus = self.set(dest, copy(&int(-1)), back);
                let plus = self.set(dest, copy(&int(1)), back);
                let nought = self.set(dest, copy(&zero), back);
                let (s, t) = self.test(BinOp::Gt, x, &zero, plus, nought);
                let above = self.block(vec![s], t);
                let (s, t) = self.test(BinOp::Lt, x, &zero, minus, above);
                (vec![s], t)
            }
            (Method::IsNegative, [x]) => (vec![assign(BinaryOp(BinOp::Lt, x.clone(), zero))], go),
            (Method::IsPositive, [x]) => (vec![assign(BinaryOp(BinOp::Gt, x.clone(), zero))], go),
            // the unchecked operation, as rustc inlines it at opt-level 2
            (Method::WrappingAdd, [a, b]) => (vec![assign(BinaryOp(BinOp::Add, a.clone(), b.clone()))], go),
            (Method::WrappingSub, [a, b]) => (vec![assign(BinaryOp(BinOp::Sub, a.clone(), b.clone()))], go),
            (Method::WrappingMul, [a, b]) => (vec![assign(BinaryOp(BinOp::Mul, a.clone(), b.clone()))], go),
            (Method::WrappingNeg, [x]) => (vec![assign(Rvalue::UnaryOp(UnOp::Neg, x.clone()))], go),
            // r = a % b; if r < 0 { r + |b| } else { r }, and `%`'s check of a divisor of zero
            (Method::RemEuclid, [a, b]) => {
                // r < 0 (a < 0): r + b, which is r + |b| when b > 0; when b < 0 it is still below 0, and less b
                // twice more is r - b
                let r = self.local(Ty::Int(ty));
                let rv = Operand::Copy(Place::local(r));
                let to_r = |v: Rvalue| Statement::Assign(Place::local(r), v);
                let done = self.set(dest, copy(&rv), back);
                let up = to_r(BinaryOp(BinOp::Add, rv.clone(), b.clone()));
                let fix = match b.int() {
                    Some((k, _)) if k > 0 => self.block(vec![up], Terminator::Goto(done)),
                    Some((k, _)) if k < 0 => {
                        let down = to_r(BinaryOp(BinOp::Sub, rv.clone(), b.clone()));
                        self.block(vec![down], Terminator::Goto(done))
                    }
                    _ => {
                        let twice = vec![
                            to_r(BinaryOp(BinOp::Sub, rv.clone(), b.clone())),
                            to_r(BinaryOp(BinOp::Sub, rv.clone(), b.clone())),
                        ];
                        let down = self.block(twice, Terminator::Goto(done));
                        let (s, t) = self.test(BinOp::Lt, &rv, &zero, down, done);
                        self.block(vec![up, s], t)
                    }
                };
                let (s, t) = self.test(BinOp::Lt, &rv, &zero, fix, done);
                let rem = to_r(BinaryOp(BinOp::Rem, a.clone(), b.clone()));
                let rest = self.block(vec![rem, s], t);
                if b.int().is_some_and(|(k, _)| k != 0) {
                    return Some((vec![], Terminator::Goto(rest)));
                }
                // `%`'s check, in every profile
                let z = self.local(Ty::Bool);
                let check = Statement::Assign(Place::local(z), BinaryOp(BinOp::Eq, b.clone(), zero));
                let assert = Terminator::Assert {
                    cond: Operand::Move(Place::local(z)),
                    expected: false,
                    msg: "attempt to calculate the remainder of `{}` with a divisor of zero".into(),
                    msg_args: vec![a.clone()],
                    target: rest,
                    unwind: Unwind::Continue,
                };
                (vec![check], assert)
            }
            // by a constant: q = a / b, and where a % b is below 0, one less for b above 0, one more below
            (Method::DivEuclid, [a, b]) => {
                let k = b.int().map(|(k, _)| k).filter(|k| *k != 0)?;
                let q = self.local(Ty::Int(ty));
                let r = self.local(Ty::Int(ty));
                let (qv, rv) = (Operand::Copy(Place::local(q)), Operand::Copy(Place::local(r)));
                let done = self.set(dest, copy(&qv), back);
                let step = if k > 0 { BinOp::Sub } else { BinOp::Add };
                let one = Operand::Constant(Const::Int { value: 1, ty });
                let fix = self.block(
                    vec![Statement::Assign(Place::local(q), BinaryOp(step, qv.clone(), one))],
                    Terminator::Goto(done),
                );
                let (s, t) = self.test(BinOp::Lt, &rv, &zero, fix, done);
                let parts = vec![
                    Statement::Assign(Place::local(q), BinaryOp(BinOp::Div, a.clone(), b.clone())),
                    Statement::Assign(Place::local(r), BinaryOp(BinOp::Rem, a.clone(), b.clone())),
                    s,
                ];
                let rest = self.block(parts, t);
                (vec![], Terminator::Goto(rest))
            }
            // Ord's: the first of two equal values is min's, the second max's (for integers, the same value)
            (Method::Min | Method::Max, [a, b]) => {
                let first = self.set(dest, copy(a), back);
                let second = self.set(dest, copy(b), back);
                let op = if m == Method::Min { BinOp::Le } else { BinOp::Gt };
                let (s, t) = self.test(op, a, b, first, second);
                (vec![s], t)
            }
            // assert!(min <= max), then x below min is min, above max is max
            (Method::Clamp, [x, lo, hi]) => {
                let low = self.set(dest, copy(lo), back);
                let high = self.set(dest, copy(hi), back);
                let keep = self.set(dest, copy(x), back);
                let (s, t) = self.test(BinOp::Gt, x, hi, high, keep);
                let above = self.block(vec![s], t);
                let (s, t) = self.test(BinOp::Lt, x, lo, low, above);
                let below = self.block(vec![s], t);
                // a constant min and max in order need no check
                if matches!((lo.int(), hi.int()), (Some((l, _)), Some((h, _))) if l <= h) {
                    return Some((vec![], Terminator::Goto(below)));
                }
                let ok = self.local(Ty::Bool);
                let check = Statement::Assign(Place::local(ok), BinaryOp(BinOp::Le, lo.clone(), hi.clone()));
                let assert = Terminator::Assert {
                    cond: Operand::Move(Place::local(ok)),
                    expected: true,
                    msg: "assertion failed: min <= max".into(),
                    msg_args: vec![],
                    target: below,
                    unwind: Unwind::Continue,
                };
                (vec![check], assert)
            }
            // the larger less the smaller
            (Method::AbsDiff, [a, b]) => {
                let up = self.set(dest, BinaryOp(BinOp::Sub, b.clone(), a.clone()), back);
                let down = self.set(dest, BinaryOp(BinOp::Sub, a.clone(), b.clone()), back);
                let (s, t) = self.test(BinOp::Lt, a, b, up, down);
                (vec![s], t)
            }
            // a - b, or 0 where that is below 0
            (Method::SaturatingSub, [a, b]) => {
                let diff = self.set(dest, BinaryOp(BinOp::Sub, a.clone(), b.clone()), back);
                let floor = self.set(dest, copy(&zero), back);
                let (s, t) = self.test(BinOp::Gt, a, b, diff, floor);
                (vec![s], t)
            }
            _ => return None,
        })
    }
}

/// Every call in `f` to one of the methods becomes the code rustc would have inlined for it.
pub fn inline_methods(f: &mut Function) {
    let next_local = f
        .params
        .iter()
        .map(|(l, _)| l.0)
        .chain(f.body.decls.iter().map(|d| d.local.0))
        .filter(|l| *l != Local::UNIT.0)
        .max()
        .map_or(0, |l| l + 1);
    let next_block = f.body.blocks.iter().map(|b| b.name.0).max().map_or(0, |b| b + 1);
    let mut gains = Gains {
        blocks: vec![],
        decls: &mut f.body.decls,
        next_local,
        next_block,
    };
    for b in f.body.blocks.iter_mut() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(back),
            unwind: Unwind::Continue | Unwind::Unreachable,
        } = &b.terminator
        else {
            continue;
        };
        let Some((m, ty)) = called(func) else {
            continue;
        };
        // each argument is read more than once now: a copy, not a move
        let args: Vec<Operand> = args
            .iter()
            .map(|a| match a {
                Operand::Move(p) => Operand::Copy(p.clone()),
                a => a.clone(),
            })
            .collect();
        let (dest, back) = (dest.clone(), *back);
        if let Some((statements, t)) = gains.expand(m, ty, &dest, &args, back) {
            b.statements.extend(statements);
            b.terminator = t;
        }
    }
    f.body.blocks.extend(gains.blocks);
}

/// Every `(a..b).contains(&x)` and `(a..=b).contains(&x)` in `f` becomes `a <= x`, then `x < b` (`x <= b`):
/// false at once where the first fails. Done after the for loops' pass has made each range a struct (`start`,
/// `end`) and before the struct pass takes it apart. Both arguments are borrows, `&range` and `&x`, each made once
/// and read only by the call; the code reads what they borrow, and they go.
pub fn inline_range_contains(f: &mut Function) {
    let next_local = f
        .params
        .iter()
        .map(|(l, _)| l.0)
        .chain(f.body.decls.iter().map(|d| d.local.0))
        .filter(|l| *l != Local::UNIT.0)
        .max()
        .map_or(0, |l| l + 1);
    let next_block = f.body.blocks.iter().map(|b| b.name.0).max().map_or(0, |b| b + 1);
    let all_blocks = f.body.blocks.clone();
    let all_statements: Vec<Statement> = all_blocks.iter().flat_map(|b| b.statements.clone()).collect();
    let mut dropped: Vec<Local> = vec![];
    let mut gains = Gains {
        blocks: vec![],
        decls: &mut f.body.decls,
        next_local,
        next_block,
    };
    for b in f.body.blocks.iter_mut() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(back),
            unwind: Unwind::Continue | Unwind::Unreachable,
        } = &b.terminator
        else {
            continue;
        };
        if let (Some((incl, ty)), [r, v]) = (contains_call(func), args.as_slice()) {
            // both are borrows, `&range` and `&x`: what they borrow, each borrow read only here
            let borrowed = |o: &Operand| -> Option<(Local, Place)> {
                let l = o.local()?;
                let mut defs = all_statements.iter().filter_map(|s| match s {
                    Statement::Assign(d, Rvalue::Ref(BorrowKind::Shared, p)) if d.as_local() == Some(l) => Some(p),
                    _ => None,
                });
                let p = defs.next()?;
                (defs.next().is_none() && super::ir::occurrences(&all_blocks, l) == 2).then(|| (l, p.clone()))
            };
            if let (Some((rl, range)), Some((vl, x))) = (borrowed(r), borrowed(v)) {
                let field = |k: u32| {
                    let mut p = range.clone();
                    p.projection.push(Projection::Field(k, Ty::Int(ty)));
                    Operand::Copy(p)
                };
                let (dest, back) = (dest.clone(), *back);
                let x = Operand::Copy(x);
                // start <= x, then x <= end (x < end): false at once where the first fails
                let upper = if incl { BinOp::Le } else { BinOp::Lt };
                let second = gains.set(&dest, Rvalue::BinaryOp(upper, x.clone(), field(1)), back);
                let no = gains.set(&dest, Rvalue::Use(Operand::Constant(Const::Bool(false))), back);
                let (s, t) = gains.test(BinOp::Le, &field(0), &x, second, no);
                b.statements.push(s);
                b.terminator = t;
                dropped.extend([rl, vl]);
            }
        }
    }
    f.body.blocks.extend(gains.blocks);
    for b in f.body.blocks.iter_mut() {
        b.statements.retain(
            |s| !matches!(s, Statement::Assign(d, Rvalue::Ref(..)) if d.as_local().is_some_and(|l| dropped.contains(&l))),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir;

    fn callee(path: &str) -> ir::Path {
        let line = format!("_3 = {path}(copy _2) -> [return: bb1, unwind continue];");
        match ir::parse_line(&line) {
            Ok(ir::Line::Terminator(Terminator::Call { func, .. })) => func,
            other => panic!("not a call: {other:?}"),
        }
    }

    #[test]
    fn the_methods_are_found_by_their_paths() {
        let known = [
            ("core::num::<impl i16>::abs", Method::Abs, IntTy::I16),
            ("core::num::<impl i16>::rem_euclid", Method::RemEuclid, IntTy::I16),
            ("core::num::<impl i16>::div_euclid", Method::DivEuclid, IntTy::I16),
            ("<i16 as Ord>::min", Method::Min, IntTy::I16),
            ("<usize as Ord>::clamp", Method::Clamp, IntTy::Usize),
            ("std::cmp::max::<u8>", Method::Max, IntTy::U8),
            ("core::cmp::min::<i16>", Method::Min, IntTy::I16),
            (
                "core::num::<impl usize>::saturating_sub",
                Method::SaturatingSub,
                IntTy::Usize,
            ),
            (
                "std::intrinsics::saturating_sub::<u8>",
                Method::SaturatingSub,
                IntTy::U8,
            ),
        ];
        for (p, m, t) in known {
            let f = callee(p);
            assert_eq!(called(&f), Some((m, t)), "{p}");
        }
        for p in [
            "core::num::<impl u8>::abs_diff::<u8>",
            "core::num::<impl i16>::abs_diff",
            "core::num::<impl i16>::pow",
            "core::num::<impl usize>::wrapping_add",
            "core::num::<impl i16>::saturating_add",
            "core::num::<impl i16>::min",
            "<i16 as PartialOrd>::lt",
            "kit::abs",
            "abs",
            "std::intrinsics::saturating_add::<usize>",
        ] {
            assert_eq!(called(&callee(p)), None, "{p}");
        }
    }

    #[test]
    fn a_call_to_abs_becomes_a_test_and_a_negation() {
        let body = ir::parse_body(
            "let mut _0: ();\n let mut _3: i16;\n bb0: {\n _3 = core::num::<impl i16>::abs(move _3) -> [return: bb1, unwind continue];\n }\n bb1: {\n return;\n }\n",
        );
        let mut f = Function {
            name: "main".into(),
            params: vec![],
            ret: Ty::Unit,
            body,
            results: vec![],
        };
        inline_methods(&mut f);
        let text: Vec<String> = f
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
                "_4 = Lt(copy _3, const 0_i16);",
                "switchInt(move _4) -> [0: bb3, otherwise: bb2];",
                "return;",
                "_3 = Neg(copy _3);",
                "goto -> bb1;",
                "_3 = copy _3;",
                "goto -> bb1;",
            ]
        );
        assert_eq!(
            f.body.decls.last().map(|d| (d.local, d.ty.clone())),
            Some((Local(4), Ty::Bool))
        );
    }
}
