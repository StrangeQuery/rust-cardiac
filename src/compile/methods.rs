//! The small integer methods of the standard library, where rustc leaves them as calls: `x.abs()`, `a.min(b)`,
//! `x.rem_euclid(7)`. At opt-level 2 and 3 rustc inlines most of them into a test or two and some arithmetic; at
//! opt-level 1 and s it calls them, across the crate boundary, and there is no MIR here to compile for the callee.
//! (`min`, `max` and `clamp` it calls at every level.) Each call becomes here what rustc would have inlined: the
//! same tests and arithmetic, as blocks of the caller's own, so every pass after sees them as it sees the program's
//! own code (the range pass knows `x.abs()` is 0 or more, say, and a check on it can go). What rustc inlines at
//! opt-level 2 is compiled as the program's own code: `signum` becomes a three-way compare, `Cmp`, which `ordering`
//! makes two tests, and a u8's `abs_diff` a subtraction in i32 and casts through a u32 (`wide`).
//!
//! What a card can't hold needs no code: no card holds `i16::MIN`, so `abs`, `/` and `%` have no overflow to check.
//! None of these methods depends on the build's overflow checks. A check that is a panic in Rust whatever the
//! profile, a divisor of zero, `clamp`'s `min <= max` and `ilog10`'s argument below 1, stays. A result that may be
//! past a card is checked in every profile, as there is no cell for the answer: an i16's `abs_diff` (999 and -999
//! are 1998 apart) and `pow`.
use super::ir::{
    BasicBlock, BinOp, Block, BorrowKind, Const, Decl, GenericArg, IntTy, Local, Operand, Path, Place, Projection,
    Rvalue, Segment, Statement, Terminator, Ty, UnOp, Unwind,
};
use super::structs::Function;

const DIVIDE_BY_ZERO: &str = "attempt to divide `{}` by zero";
const REMAINDER_BY_ZERO: &str = "attempt to calculate the remainder of `{}` with a divisor of zero";

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
    SaturatingAdd,
    Midpoint,
    Ilog10,
    WrappingDiv,
    OverflowingDiv,
    Pow,
}

/// The types it compiles them for: the card's own number for the signed methods, and every integer type for the
/// ones that only compare (`min`, `max`, `clamp`) or can't leave the type's range (`abs_diff`, `saturating_sub` of
/// an unsigned type). A u8's `saturating_add` and `midpoint` work in an i16, whose cell holds the sum of two u8s.
/// The rest, on u8 or usize, wrap or saturate where a card does not (at 256, at 2^64). On an i16, `abs_diff` and
/// `unsigned_abs` return a u16 and `ilog10` a u32, which are cells for what these return (`wide`).
fn method(name: &str, ty: IntTy) -> Option<Method> {
    use Method::*;
    let signed = ty == IntTy::I16;
    let unsigned = matches!(ty, IntTy::U8 | IntTy::Usize);
    Some(match name {
        "abs" | "wrapping_abs" | "unsigned_abs" if signed => Abs,
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
        "abs_diff" if signed || unsigned => AbsDiff,
        "saturating_sub" if unsigned => SaturatingSub,
        "saturating_add" if ty == IntTy::U8 => SaturatingAdd,
        "midpoint" if ty == IntTy::U8 => Midpoint,
        "ilog10" if signed => Ilog10,
        "wrapping_div" if signed => WrappingDiv,
        "overflowing_div" if signed => OverflowingDiv,
        "pow" if signed => Pow,
        _ => return None,
    })
}

/// The method a call is to, and the integer type it is on, from the callee's path: `core::num::<impl i16>::abs`,
/// `<u8 as Ord>::min`, `core::cmp::max::<usize>`, or `std::intrinsics::saturating_sub::<usize>`, which is what
/// `saturating_sub` is at opt-level 2 (and `saturating_add` likewise).
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
            (
                method(m, t).filter(|m| matches!(m, Method::SaturatingSub | Method::SaturatingAdd))?,
                t,
            )
        }
        _ => return None,
    };
    Some((m, ty))
}

/// The bounds of what a call to one of these methods returns in a u16 or a u32, where it is on a card (its code
/// here makes sure of that): the cells `wide` may hold such a result in.
pub fn exact_result(func: &Path) -> Option<[i128; 2]> {
    match called(func)? {
        (Method::Abs | Method::AbsDiff, IntTy::I16) => Some([0, 999]),
        (Method::Ilog10, IntTy::I16) => Some([0, 2]),
        _ => None,
    }
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
pub(super) struct Gains<'a> {
    pub(super) blocks: Vec<Block>,
    decls: &'a mut Vec<Decl>,
    next_local: u32,
    next_block: u32,
}

impl<'a> Gains<'a> {
    /// For a function with these parameters and blocks: each new local and block is numbered past its own.
    pub(super) fn new(params: &[(Local, Ty)], blocks: &[Block], decls: &'a mut Vec<Decl>) -> Gains<'a> {
        let next_local = params
            .iter()
            .map(|(l, _)| l.0)
            .chain(decls.iter().map(|d| d.local.0))
            .filter(|l| *l != Local::UNIT.0)
            .max()
            .map_or(0, |l| l + 1);
        let next_block = blocks.iter().map(|b| b.name.0).max().map_or(0, |b| b + 1);
        Gains {
            blocks: vec![],
            decls,
            next_local,
            next_block,
        }
    }

    pub(super) fn local(&mut self, ty: Ty) -> Local {
        let l = Local(self.next_local);
        self.next_local += 1;
        self.decls.push(Decl {
            local: l,
            mutable: true,
            ty,
        });
        l
    }

    pub(super) fn block(&mut self, statements: Vec<Statement>, terminator: Terminator) -> BasicBlock {
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
    pub(super) fn test(
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

    /// `t = op(a, b)` overflow-checked, as rustc writes it: the statement, and the assert of its flag, which goes
    /// on to a block that sets `dest` from the pair and goes to `back`.
    fn checked(
        &mut self,
        op: BinOp,
        a: &Operand,
        b: &Operand,
        dest: &Place,
        back: BasicBlock,
    ) -> (Statement, Terminator) {
        let pair = self.local(Ty::Tuple(vec![Ty::Int(IntTy::I16), Ty::Bool]));
        let field = |k: u32, t: Ty| Place {
            base: Place::local(pair).base,
            projection: vec![Projection::Field(k, t)],
        };
        let value = Rvalue::Use(Operand::Move(field(0, Ty::Int(IntTy::I16))));
        let set = self.set(dest, value, back);
        let msg = match op {
            BinOp::SubWithOverflow => "attempt to compute `{} - {}`, which would overflow",
            _ => "attempt to compute `{} * {}`, which would overflow",
        };
        (
            Statement::Assign(Place::local(pair), Rvalue::BinaryOp(op, a.clone(), b.clone())),
            Terminator::Assert {
                cond: Operand::Move(field(1, Ty::Bool)),
                expected: false,
                msg: msg.into(),
                msg_args: vec![a.clone(), b.clone()],
                target: set,
                unwind: Unwind::Continue,
            },
        )
    }

    /// A check that `b` isn't zero, as `/` and `%` have in every profile, going on to `then`: the statement and the
    /// assert.
    fn nonzero(&mut self, a: &Operand, b: &Operand, ty: IntTy, msg: &str, then: BasicBlock) -> (Statement, Terminator) {
        let z = self.local(Ty::Bool);
        let zero = Operand::Constant(Const::Int { value: 0, ty });
        (
            Statement::Assign(Place::local(z), Rvalue::BinaryOp(BinOp::Eq, b.clone(), zero)),
            Terminator::Assert {
                cond: Operand::Move(Place::local(z)),
                expected: false,
                msg: msg.into(),
                msg_args: vec![a.clone()],
                target: then,
                unwind: Unwind::Continue,
            },
        )
    }

    /// A block that sets the call's destination and goes back to where the call went on to.
    pub(super) fn set(&mut self, dest: &Place, v: Rvalue, back: BasicBlock) -> BasicBlock {
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
        let one = int(1);
        // a card's own number, where a u8's method works in one
        let card = |v: i128| {
            Operand::Constant(Const::Int {
                value: v,
                ty: IntTy::I16,
            })
        };
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
                let (check, assert) = self.nonzero(a, b, ty, REMAINDER_BY_ZERO, rest);
                (vec![check], assert)
            }
            // q = a / b, and where a % b is below 0, one less for b above 0, one more below; by a variable, with
            // `/`'s check (no card holds i16::MIN, so `/` has no overflow to check)
            (Method::DivEuclid, [a, b]) => {
                let q = self.local(Ty::Int(ty));
                let r = self.local(Ty::Int(ty));
                let (qv, rv) = (Operand::Copy(Place::local(q)), Operand::Copy(Place::local(r)));
                let done = self.set(dest, copy(&qv), back);
                let mut step = |op: BinOp| {
                    let s = Statement::Assign(Place::local(q), BinaryOp(op, qv.clone(), int(1)));
                    self.block(vec![s], Terminator::Goto(done))
                };
                let fix = match b.int() {
                    Some((0, _)) => return None,
                    Some((k, _)) => step(if k > 0 { BinOp::Sub } else { BinOp::Add }),
                    None => {
                        let (down, up) = (step(BinOp::Sub), step(BinOp::Add));
                        let (s, t) = self.test(BinOp::Gt, b, &zero, down, up);
                        self.block(vec![s], t)
                    }
                };
                let (s, t) = self.test(BinOp::Lt, &rv, &zero, fix, done);
                let parts = vec![
                    Statement::Assign(Place::local(q), BinaryOp(BinOp::Div, a.clone(), b.clone())),
                    Statement::Assign(Place::local(r), BinaryOp(BinOp::Rem, a.clone(), b.clone())),
                    s,
                ];
                let rest = self.block(parts, t);
                if b.int().is_some() {
                    return Some((vec![], Terminator::Goto(rest)));
                }
                let (check, assert) = self.nonzero(a, b, ty, DIVIDE_BY_ZERO, rest);
                (vec![check], assert)
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
            // the larger less the smaller; of two i16s that may be past a card (999 - -999), so it is checked
            (Method::AbsDiff, [a, b]) if ty == IntTy::I16 => {
                let (s, t) = self.checked(BinOp::SubWithOverflow, b, a, dest, back);
                let up = self.block(vec![s], t);
                let (s, t) = self.checked(BinOp::SubWithOverflow, a, b, dest, back);
                let down = self.block(vec![s], t);
                let (s, t) = self.test(BinOp::Lt, a, b, up, down);
                (vec![s], t)
            }
            (Method::AbsDiff, [a, b]) => {
                let up = self.set(dest, BinaryOp(BinOp::Sub, b.clone(), a.clone()), back);
                let down = self.set(dest, BinaryOp(BinOp::Sub, a.clone(), b.clone()), back);
                let (s, t) = self.test(BinOp::Lt, a, b, up, down);
                (vec![s], t)
            }
            // a + b in an i16, which holds it (at most 510), or 255 where that is past a u8
            (Method::SaturatingAdd, [a, b]) => {
                let sum = self.local(Ty::Int(IntTy::I16));
                let sv = Operand::Copy(Place::local(sum));
                let top = self.set(dest, copy(&int(255)), back);
                let keep = self.set(dest, copy(&sv), back);
                let (s, t) = self.test(BinOp::Gt, &sv, &card(255), top, keep);
                let add = Statement::Assign(Place::local(sum), BinaryOp(BinOp::Add, a.clone(), b.clone()));
                (vec![add, s], t)
            }
            // (a + b) / 2, rounded down, the sum in an i16, which holds it
            (Method::Midpoint, [a, b]) => {
                let sum = self.local(Ty::Int(IntTy::I16));
                let sv = Operand::Copy(Place::local(sum));
                let add = Statement::Assign(Place::local(sum), BinaryOp(BinOp::Add, a.clone(), b.clone()));
                (vec![add, assign(BinaryOp(BinOp::Div, sv, card(2)))], go)
            }
            // a panic below 1, as in Rust; then 0, 1 or 2, by tests against 10 and 100 (a card is below 1000). The
            // u32 result is an i16 by now (`wide`)
            (Method::Ilog10, [x]) => {
                let out = |v: i128| Use(card(v));
                let (one, two) = (self.set(dest, out(1), back), self.set(dest, out(2), back));
                let nought = self.set(dest, out(0), back);
                let (s, t) = self.test(BinOp::Lt, x, &int(100), one, two);
                let tens = self.block(vec![s], t);
                let (s, t) = self.test(BinOp::Lt, x, &int(10), nought, tens);
                let rest = self.block(vec![s], t);
                let ok = self.local(Ty::Bool);
                let check = Statement::Assign(Place::local(ok), BinaryOp(BinOp::Gt, x.clone(), zero));
                let assert = Terminator::Assert {
                    cond: Operand::Move(Place::local(ok)),
                    expected: true,
                    msg: "argument of integer logarithm must be positive".into(),
                    msg_args: vec![],
                    target: rest,
                    unwind: Unwind::Continue,
                };
                (vec![check], assert)
            }
            // `/` with its check of b; no card holds i16::MIN, so it never wraps
            (Method::WrappingDiv, [a, b]) => {
                let quotient = self.set(dest, BinaryOp(BinOp::Div, a.clone(), b.clone()), back);
                let (check, assert) = self.nonzero(a, b, ty, DIVIDE_BY_ZERO, quotient);
                (vec![check], assert)
            }
            // the same, in the pair's first field, and false in its second
            (Method::OverflowingDiv, [a, b]) => {
                let field = |k: u32, t: Ty| {
                    let mut p = dest.clone();
                    p.projection.push(Projection::Field(k, t));
                    p
                };
                let parts = vec![
                    Statement::Assign(field(0, Ty::Int(ty)), BinaryOp(BinOp::Div, a.clone(), b.clone())),
                    Statement::Assign(field(1, Ty::Bool), Use(Operand::Constant(Const::Bool(false)))),
                ];
                let quotient = self.block(parts, go);
                let (check, assert) = self.nonzero(a, b, ty, DIVIDE_BY_ZERO, quotient);
                (vec![check], assert)
            }
            // by a constant k: k - 1 multiplications, each checked in every profile, as a product past a card can't
            // be held (with overflow checks off Rust's would wrap at 32768)
            (Method::Pow, [x, k]) => {
                let k = k.int().map(|(k, _)| k).filter(|k| (0..=10).contains(k))?;
                if k <= 1 {
                    return Some((vec![assign(copy(if k == 0 { &one } else { x }))], go));
                }
                // r2 = x * x, r3 = r2 * x, .. into dest: built from the last
                let (mut next, mut into) = (back, dest.clone());
                for i in (1..k).rev() {
                    let r = (i > 1).then(|| Place::local(self.local(Ty::Int(ty))));
                    let left = r.clone().map_or(x.clone(), Operand::Copy);
                    let (s, t) = self.checked(BinOp::MulWithOverflow, &left, x, &into, next);
                    next = self.block(vec![s], t);
                    into = r.unwrap_or(into);
                }
                (vec![], Terminator::Goto(next))
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
    inline(f, false);
}

/// Every call in `f` to a method that returns a pair, `overflowing_div`'s `(i16, bool)`: done before the struct
/// pass, which takes the pair apart into its fields' cells.
pub fn inline_pair_methods(f: &mut Function) {
    inline(f, true);
}

fn inline(f: &mut Function, pairs: bool) {
    let mut gains = Gains::new(&f.params, &f.body.blocks, &mut f.body.decls);
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
        let Some((m, ty)) = called(func).filter(|(m, _)| (*m == Method::OverflowingDiv) == pairs) else {
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
    let all_blocks = f.body.blocks.clone();
    let all_statements: Vec<Statement> = all_blocks.iter().flat_map(|b| b.statements.clone()).collect();
    let mut dropped: Vec<Local> = vec![];
    let mut gains = Gains::new(&f.params, &f.body.blocks, &mut f.body.decls);
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
            ("core::num::<impl i16>::abs_diff", Method::AbsDiff, IntTy::I16),
            ("core::num::<impl i16>::unsigned_abs", Method::Abs, IntTy::I16),
            ("core::num::<impl i16>::pow", Method::Pow, IntTy::I16),
            (
                "core::intrinsics::saturating_add::<u8>",
                Method::SaturatingAdd,
                IntTy::U8,
            ),
        ];
        for (p, m, t) in known {
            let f = callee(p);
            assert_eq!(called(&f), Some((m, t)), "{p}");
        }
        for p in [
            "core::num::<impl u8>::abs_diff::<u8>",
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
