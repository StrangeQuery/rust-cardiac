//! Arithmetic in the accumulator: sums, multiplies (by adds and decimal shifts, or the shared routine), divides
//! (by shifts, by adds, by subtracting, or the shared routine), and the range checks on the results.
use crate::compile::ir::{self, BinOp, IntTy, Projection, Ty};
use crate::compile::machine::Op;
use crate::compile::Fail;

use super::places::{range_of, Loc, Value};
use super::symbols::shift;
use super::{Cell, Lowering, Sym};
use crate::compile::runtime::{BitRoutine, Bitwise};

/// The routine a shift by a variable calls.
enum Shift {
    Shr,
    Shl,
    Shl8,
}

/// The plain operation an overflow-checked one does.
pub(super) fn plain_op(op: BinOp) -> BinOp {
    match op {
        BinOp::AddWithOverflow => BinOp::Add,
        BinOp::SubWithOverflow => BinOp::Sub,
        _ => BinOp::Mul,
    }
}

impl<'a> Lowering<'a> {
    fn use_mul(&mut self) {
        self.sh.pool.mul = true;
        self.konst(0);
    }

    fn use_div(&mut self) {
        self.sh.pool.div = true;
        self.konst(0);
    }

    /// The type of a place: its local's, through its projections.
    pub(super) fn place_ty(&self, p: &ir::Place) -> Option<Ty> {
        let mut t = self.locals.get(&p.base_local()?)?.clone();
        for x in &p.projection {
            t = match (x, t) {
                (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
                (Projection::Field(_, f), _) => f.clone(),
                (
                    Projection::Index(_)
                    | Projection::IndexPlus(..)
                    | Projection::ConstantIndex { .. }
                    | Projection::IndexPlace(_),
                    Ty::Array(e, _),
                ) => *e,
                _ => return None,
            };
        }
        Some(t)
    }

    /// The type of an operand: a constant's own, or its place's.
    pub(super) fn operand_ty(&self, o: &ir::Operand) -> Option<Ty> {
        match o.int() {
            Some((_, t)) => Some(Ty::Int(t)),
            None => self.place_ty(o.place()?),
        }
    }

    /// `x & m` on an integer, m one below a power of two: x.rem_euclid(m + 1), as two's complement has it (a usize
    /// past a card, held below 0, too).
    pub(super) fn mask(&mut self, src: &str, dest: &ir::Place, x: &ir::Operand, m: i64) -> Result<(), Fail> {
        let dst = self.place_any(dest)?;
        self.mask_acc(src, x, m)?;
        self.store(&dst, "")
    }

    /// `x & m` into the accumulator. Subtract m + 1 until the accumulator goes negative, then step back once; a
    /// negative x first has m + 1 added until it is not.
    fn mask_acc(&mut self, src: &str, x: &ir::Operand, m: i64) -> Result<(), Fail> {
        let a = self.operand(x)?;
        let a = self.mat(a);
        if m == 0 {
            let z = self.konst(0);
            self.emit_c(Op::Cla, z, src);
            return Ok(());
        }
        // a u8, or a usize that can't be past a card, is never below 0
        let never_negative = match self.operand_ty(x) {
            Some(Ty::Int(IntTy::U8)) => true,
            Some(Ty::Int(IntTy::Usize)) => !self.is_huge(x),
            _ => false,
        };
        let k = self.konst(m + 1);
        let (test, lp, up, done) = (self.fresh("neg"), self.fresh("mask"), self.fresh("up"), self.fresh("d"));
        self.load(&a)?;
        if !never_negative {
            self.define(&test);
            self.emit_c(Op::Tac, up.clone(), src);
        }
        self.define(&lp);
        self.emit_c(Op::Sub, k.clone(), if never_negative { src } else { "" });
        self.emit(Op::Tac, done.clone());
        self.emit(Op::Jmp, lp);
        if !never_negative {
            self.define(&up);
            self.emit(Op::Add, k.clone());
            self.emit(Op::Jmp, test);
        }
        self.define(&done);
        self.emit(Op::Add, k);
        Ok(())
    }

    /// `x & y`, `x | y` or `x ^ y` of two integers by the routine for it (`runtime::bitwise`): on a card, as two's
    /// complement has it, whatever the type (a u8's and an i8's bits are an i16's too); past a card it panics.
    pub(super) fn bitwise(
        &mut self,
        src: &str,
        dest: &ir::Place,
        op: BinOp,
        x: &ir::Operand,
        y: &ir::Operand,
    ) -> Result<(), Fail> {
        let on_a_card = |l: &Self, o: &ir::Operand| match l.operand_ty(o) {
            Some(Ty::Int(IntTy::I16 | IntTy::I8 | IntTy::U8)) => true,
            Some(Ty::Int(IntTy::Usize)) => !l.is_huge(o),
            _ => false,
        };
        if !on_a_card(self, x) || !on_a_card(self, y) {
            fail!(operator_type(op.name(), src))
        }
        let op = match op {
            BinOp::BitAnd => Bitwise::And,
            BinOp::BitOr => Bitwise::Or,
            _ => Bitwise::Xor,
        };
        // two u8s are never below 0, and stay below 256
        let byte = [x, y].iter().all(|o| self.operand_ty(o) == Some(Ty::Int(IntTy::U8)));
        let routine = BitRoutine { op, byte };
        if !self.sh.pool.bitwise.contains(&routine) {
            self.sh.pool.bitwise.push(routine);
        }
        let [entry, ca, cb] = routine.cells();
        let dst = self.place_any(dest)?;
        let a = self.operand(x)?;
        let a = self.mat(a);
        let b = self.operand(y)?;
        let b = self.mat(b);
        let at = self.code.len();
        self.pass(&a, &Sym::Runtime(ca), src)?;
        self.pass(&b, &Sym::Runtime(cb), "")?;
        let comment = if self.code.len() == at { src } else { "" };
        self.emit_c(Op::Jmp, Sym::Runtime(entry), comment);
        let r = self.fresh("rr");
        self.define(&r);
        self.store(&dst, "")
    }

    /// `x << n` or `x >> n` by a routine (`runtime::shr`, `shl` and `shl8`): n is taken modulo the type's bits first,
    /// as Rust's operators take it once their overflow check has passed (and without one, as `wrapping_shl` does).
    /// `>>` of an i16, an i8, a u8 or a usize on a card; `<<` of an i16, which panics past a card, and of a u8.
    pub(super) fn shift(
        &mut self,
        src: &str,
        dest: &ir::Place,
        left: bool,
        x: &ir::Operand,
        y: &ir::Operand,
    ) -> Result<(), Fail> {
        let name = if left { "<<" } else { ">>" };
        let ty = match self.operand_ty(x) {
            Some(Ty::Int(t)) => t,
            _ => fail!(method_type(name, src)),
        };
        let (routine, bits) = match (left, ty) {
            (false, IntTy::I16) => (Shift::Shr, 16),
            (false, IntTy::I8 | IntTy::U8) => (Shift::Shr, 8),
            (false, IntTy::Usize) if !self.is_huge(x) => (Shift::Shr, 64),
            (true, IntTy::I16) => (Shift::Shl, 16),
            (true, IntTy::U8) => (Shift::Shl8, 8),
            _ => fail!(shift_type(name, src)),
        };
        let [entry, ca] = match routine {
            Shift::Shr => {
                self.sh.pool.shr = true;
                ["rt_shr_entry", "rt_shr_a"]
            }
            Shift::Shl => {
                self.sh.pool.shl = true;
                ["rt_shl_entry", "rt_shl_a"]
            }
            Shift::Shl8 => {
                self.sh.pool.shl8 = true;
                ["rt_shl8_entry", "rt_shl8_a"]
            }
        };
        let dst = self.place_any(dest)?;
        let a = self.operand(x)?;
        let a = self.mat(a);
        self.pass(&a, &Sym::Runtime(ca), src)?;
        match y.int() {
            Some((k, _)) => {
                let n = self.konst(k.rem_euclid(bits) as i64);
                self.emit(Op::Cla, n);
            }
            None => self.mask_acc("", y, bits as i64 - 1)?,
        }
        self.emit(Op::Jmp, Sym::Runtime(entry));
        let r = self.fresh("rr");
        self.define(&r);
        self.store(&dst, "")
    }

    /// a / b or a % b. The divisor is never zero here, and never -1 with i16::MIN: rustc's own asserts run first.
    pub(super) fn divide(
        &mut self,
        src: &str,
        dest: &ir::Place,
        rem: bool,
        x: &ir::Operand,
        y: &ir::Operand,
        unsigned: bool,
    ) -> Result<(), Fail> {
        let dst = self.place_any(dest)?;
        let a = self.operand(x)?;
        let a = self.mat(a);
        let b = self.operand(y)?;
        let b = self.mat(b);
        // the operation's type: the destination's (an element's, say), else an operand's
        let ty = self
            .place_ty(dest)
            .or_else(|| [x, y].iter().find_map(|o| self.operand_ty(o)));
        let signed = matches!(ty, Some(Ty::Int(IntTy::I16 | IntTy::I8)));
        let p10 = match b.k() {
            Some(k) => [10, 100].iter().position(|&p| p == k.abs()).map_or(0, |i| i as i64 + 1),
            None => 0,
        };
        if p10 > 0 {
            // by 10 or 100: a decimal shift right, which truncates toward zero as Rust's / does; the remainder is what
            // the shift dropped, with the dividend's sign, as Rust's % is (SFT 33 keeps the last digit, SFT 22 the
            // last two)
            self.load(&a)?;
            if rem {
                self.emit_c(Op::Sft, shift(4 - p10, 4 - p10), src)
            } else {
                self.emit_c(Op::Sft, shift(0, p10), src)
            }
            if !rem && b.k().unwrap() < 0 {
                let t = self.temp();
                self.emit(Op::Sto, t.clone());
                let z = self.konst(0);
                self.emit(Op::Cla, z);
                self.emit(Op::Sub, t);
            }
            return self.store(&dst, "");
        }
        let by_adds = match b.k() {
            Some(2) => Some((5, 1)),
            Some(5) => Some((2, 1)),
            Some(20) => Some((5, 2)),
            Some(25) => Some((4, 2)),
            Some(50) => Some((2, 2)),
            _ => None,
        };
        if let Some((times, places)) = by_adds {
            if !rem {
                // a / d for d dividing a power of ten: a (10^p / d) / 10^p, multiplied up by adds in the accumulator
                // (at most 5 x 999, which four digits hold) and shifted back: exact, and toward zero as Rust's / is
                self.load(&a)?;
                for k in 1..times {
                    let v = self.arith(&a)?;
                    self.emit_c(Op::Add, v, if k == 1 { src } else { "" });
                }
                self.emit(Op::Sft, shift(0, places));
                return self.store(&dst, "");
            }
        }
        if signed && !unsigned {
            // signed: the shared routine
            self.use_div();
            let at = self.code.len();
            self.pass(&a, &Sym::Runtime("rt_div_a"), src)?;
            self.pass(&b, &Sym::Runtime("rt_div_b"), "")?;
            let comment = if self.code.len() == at { src } else { "" };
            self.emit_c(Op::Jmp, Sym::Runtime("rt_div_entry"), comment);
            let r = self.fresh("rr");
            self.define(&r);
            if rem {
                self.emit(Op::Cla, Sym::Runtime("rt_div_r"));
                self.sh.pool.divr = true;
            }
            return self.store(&dst, "");
        }
        // unsigned: subtract until the accumulator goes negative, then step back once
        let (lp, done) = (self.fresh("div"), self.fresh("d"));
        if rem {
            self.load(&a)?;
            self.define(&lp);
            let v = self.arith(&b)?;
            self.emit_c(Op::Sub, v, src);
            self.emit(Op::Tac, done.clone());
            self.emit(Op::Jmp, lp);
            self.define(&done);
            let v = self.arith(&b)?;
            self.emit(Op::Add, v);
            return self.store(&dst, "");
        }
        // the quotient is counted in its own cell, or in a scratch cell and then stored there: for a computed place
        // (an element at a variable index, through a reference), and for the dividend's or the divisor's own cell,
        // which the count would change while the loop still reads it
        let operand = |v: &Value, d: &Cell| {
            let Value::At(c) = v else { return false };
            c.sym == d.sym && c.offset.unwrap_or(0) == d.offset.unwrap_or(0)
        };
        let (q, own) = match &dst {
            Loc::At(d) if !operand(&a, d) && !operand(&b, d) => (d.clone(), true),
            _ => (self.temp().into(), false),
        };
        // the running remainder, in a cell of its own
        let r = Sym::Fresh {
            func: self.func.clone(),
            stem: "divr",
            n: self.tmp,
        };
        self.cells.insert(r.clone());
        let z = self.konst(0);
        self.emit_c(Op::Cla, z, src);
        self.emit(Op::Sto, q.clone());
        self.load(&a)?;
        self.define(&lp);
        let v = self.arith(&b)?;
        self.emit(Op::Sub, v);
        self.emit(Op::Tac, done.clone());
        self.emit(Op::Sto, r.clone());
        self.emit(Op::Cla, q.clone());
        let one = self.konst(1);
        self.emit(Op::Add, one);
        self.emit(Op::Sto, q.clone());
        self.emit(Op::Cla, r);
        self.emit(Op::Jmp, lp);
        self.define(&done);
        if own {
            return Ok(());
        }
        self.emit(Op::Cla, q);
        self.store(&dst, "")
    }

    /// Range check on the accumulator for type ty (the value is still whole there: it holds four digits): falls
    /// through when it fits, jumps to fail when it does not. No card holds 1000 or more, so the top check is
    /// value - 999 - 1 < 0. Changes the accumulator.
    pub(super) fn check_acc(&mut self, ty: Option<IntTy>, fail: &Sym, op: BinOp) -> Result<(), Fail> {
        let Some((lo, hi)) = ty.and_then(range_of) else {
            fail!(internal(format!(
                "no range for the type {}",
                ty.map_or("undefined", IntTy::name)
            )))
        };
        let ok = self.fresh("rc");
        // unsigned: a sum of two can only be too big, a difference only too small
        let low = !(lo == 0 && op == BinOp::Add);
        let high = !(lo == 0 && op == BinOp::Sub);
        if low {
            if lo == 0 {
                self.emit(Op::Tac, fail.clone());
            } else {
                let c = self.konst(-lo);
                self.emit(Op::Add, c);
                self.emit(Op::Tac, fail.clone());
                if high {
                    let c = self.konst(-lo);
                    self.emit(Op::Sub, c);
                }
            }
        }
        if high {
            if hi < 999 {
                let c = self.konst(hi + 1);
                self.emit(Op::Sub, c);
            } else {
                let c = self.konst(hi);
                self.emit(Op::Sub, c);
                let one = self.konst(1);
                self.emit(Op::Sub, one);
            }
            self.emit(Op::Tac, ok.clone());
            self.emit(Op::Jmp, fail.clone());
            self.define(&ok);
        }
        Ok(())
    }

    /// The accumulator mod 256, as a u8 keeps it: from a difference of two u8s (-255..=255) with `low`, one 256
    /// added back to a negative one; else from a value not negative, 256 taken away while it lasts.
    pub(super) fn wrap_u8(&mut self, low: bool) {
        let k = self.konst(256);
        let done = self.fresh("d");
        if low {
            let up = self.fresh("wrap");
            self.emit(Op::Tac, up.clone());
            self.emit(Op::Jmp, done.clone());
            self.define(&up);
            self.emit(Op::Add, k);
            self.define(&done);
        } else {
            let lp = self.fresh("wrap");
            self.define(&lp);
            self.emit(Op::Sub, k.clone());
            self.emit(Op::Tac, done.clone());
            self.emit(Op::Jmp, lp);
            self.define(&done);
            self.emit(Op::Add, k);
        }
    }

    /// The accumulator as an i8 keeps it, mod 256 from -128 to 127: from anything a cell or a sum of two holds,
    /// 256 added while it is below -128, and taken away while it is above 127.
    pub(super) fn wrap_i8(&mut self) {
        let (k256, k128) = (self.konst(256), self.konst(128));
        let (up, down, back, done) = (
            self.fresh("wrap"),
            self.fresh("wrap"),
            self.fresh("wrap"),
            self.fresh("d"),
        );
        // with 128 added the wanted range is 0..=255
        self.emit(Op::Add, k128.clone());
        self.define(&up);
        self.emit(Op::Tac, back.clone());
        self.define(&down);
        self.emit(Op::Sub, k256.clone());
        self.emit(Op::Tac, done.clone());
        self.emit(Op::Jmp, down);
        self.define(&back);
        self.emit(Op::Add, k256.clone());
        self.emit(Op::Jmp, up);
        self.define(&done);
        self.emit(Op::Add, k256);
        self.emit(Op::Sub, k128);
    }

    /// The signed terms of a sum into the accumulator.
    pub(super) fn sum_acc(&mut self, terms: &[(ir::Sign, ir::Operand)], comment: &str) -> Result<(), Fail> {
        // every term in a cell first: one read through an address is read through the accumulator, which the
        // partial sum would be in
        let mut values = vec![];
        for (sign, o) in terms {
            let v = self.operand(o)?;
            values.push((*sign, self.mat(v)));
        }
        for (i, (sign, v)) in values.into_iter().enumerate() {
            let plus = sign == ir::Sign::Plus;
            if i == 0 && plus {
                self.load(&v)?;
            } else {
                if i == 0 {
                    let z = self.konst(0);
                    self.emit(Op::Cla, z);
                }
                let x = self.arith(&v)?;
                self.emit_c(
                    if plus { Op::Add } else { Op::Sub },
                    x,
                    if i == 1 { comment } else { "" },
                );
            }
        }
        Ok(())
    }

    /// a op b into the accumulator. Overflow-checked (`check`: where to go on overflow), a multiply whose partial
    /// result could pass four digits is checked on the way; the caller checks the final value.
    pub(super) fn arith_acc(
        &mut self,
        op: BinOp,
        a: Value,
        b: Value,
        check: Option<&Sym>,
        comment: &str,
    ) -> Result<(), Fail> {
        if op == BinOp::Add || op == BinOp::Sub {
            self.load(&a)?;
            let v = self.arith(&b)?;
            self.emit_c(if op == BinOp::Add { Op::Add } else { Op::Sub }, v, comment);
            return Ok(());
        }
        // Mul: by a constant, adds and decimal shifts; otherwise the shared routine (repeated addition)
        let (a, b) = if a.k().is_some() && b.k().is_none() {
            (b, a)
        } else {
            (a, b)
        };
        let Some(c) = b.k() else {
            return self.mul_call(&a, &b, check.is_some(), comment);
        };
        let (n, neg) = (c.abs(), c < 0);
        if n == 0 {
            let z = self.konst(0);
            self.emit_c(Op::Cla, z, comment);
            return Ok(());
        }
        if let Some(ak) = a.k() {
            // both known (it fits: rustc would have folded it otherwise)
            let v = self.konst(ak * c);
            self.emit_c(Op::Cla, v, comment);
            return Ok(());
        }
        let (d2, d1, d0) = (n / 100, (n / 10) % 10, n % 10);
        if n >= 100 && !(n == 100 || n == 200) {
            return self.mul_call(&a, &b, check.is_some(), comment); // a big constant: the routine
        }
        // by a negative constant the product is taken away from 0, so the accumulator holds it whole, as the check
        // after it reads it (stored and negated, a product past a card would keep three digits)
        if n >= 100 {
            self.times(&a, d2, true, neg)?;
            if let Some(fail) = check {
                self.check99(fail);
            }
            self.emit(Op::Sft, shift(1, 0));
            self.emit(Op::Sft, shift(1, 0));
        } else if n >= 10 {
            self.times(&a, d1, true, neg)?;
            if let Some(fail) = check {
                self.check99(fail);
            }
            self.emit(Op::Sft, shift(1, 0));
            self.times(&a, d0, false, neg)?;
        } else if n >= 7 {
            // 7, 8 or 9 times: ten times, less the rest (`SFT 10 / SUB a` for 9). Ten times a card is four digits
            // at most, which the accumulator holds, so no check is needed before the shift
            self.times(&a, 1, true, neg)?;
            self.emit(Op::Sft, shift(1, 0));
            self.times(&a, 10 - n, false, !neg)?;
        } else {
            self.times(&a, d0, true, neg)?;
        }
        Ok(())
    }

    /// a added up k times into the accumulator (loaded first when `first`), or taken away k times when `neg` (from
    /// 0 when `first`).
    fn times(&mut self, a: &Value, k: i64, first: bool, neg: bool) -> Result<(), Fail> {
        if first && neg {
            // a value carried in the accumulator is read from its cell here, so it goes there first
            if matches!(a, Value::At(c) if self.carried.as_ref() == Some(c)) {
                self.carried = None;
                let x = self.arith(a)?;
                self.emit(Op::Sto, x);
            }
            let z = self.konst(0);
            self.emit(Op::Cla, z);
        }
        for i in 0..k {
            if i == 0 && first && !neg {
                self.load(a)?;
            } else {
                let v = self.arith(a)?;
                self.emit(if neg { Op::Sub } else { Op::Add }, v);
            }
        }
        Ok(())
    }

    /// Before a decimal shift left: the accumulator must be within +-99, else the shift would carry past four digits.
    fn check99(&mut self, fail: &Sym) {
        let ok = self.fresh("rc");
        let k99 = self.konst(99);
        self.emit(Op::Add, k99.clone());
        self.emit(Op::Tac, fail.clone());
        self.emit(Op::Sub, k99.clone());
        self.emit(Op::Sub, k99);
        let one = self.konst(1);
        self.emit(Op::Sub, one);
        self.emit(Op::Tac, ok.clone());
        self.emit(Op::Jmp, fail.clone());
        self.define(&ok);
        let k100 = self.konst(100);
        self.emit(Op::Add, k100);
    }

    /// a * b by the shared routine.
    pub(super) fn mul_call(&mut self, a: &Value, b: &Value, checked: bool, comment: &str) -> Result<(), Fail> {
        self.use_mul();
        if checked {
            self.sh.pool.mul_checked = true;
        }
        let at = self.code.len();
        self.pass(a, &Sym::Runtime("rt_mul_a"), comment)?;
        self.pass(b, &Sym::Runtime("rt_mul_b"), "")?;
        let comment = if self.code.len() == at { comment } else { "" };
        self.emit_c(Op::Jmp, Sym::Runtime("rt_mul_entry"), comment);
        let r = self.fresh("rr");
        self.define(&r);
        Ok(())
    }
}
