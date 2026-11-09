//! Statements: each assignment, by the shape of its right-hand side, and the constants main's opening blocks store
//! that become cells' initial values instead.
use crate::compile::ir::{self, BinOp, Const, IntTy, Local, Locals, Projection, Rvalue, Statement, Ty, UnOp};
use crate::compile::machine::Op;
use crate::compile::runtime::NO_CARD;
use crate::compile::Fail;

use super::arith::plain_op;
use super::places::{Loc, Value};
use super::{is_comparison, numbered, one_word, source, Access, Cell, Lowering, Operand, Sym};

/// A value an array repeat stores into each element: a scalar constant, or a place read without indexing (rustc
/// reads an element into a temporary first).
fn repeatable(o: &ir::Operand) -> bool {
    match o {
        ir::Operand::Constant(c) => c.is_scalar(),
        ir::Operand::Copy(p) | ir::Operand::Move(p) | ir::Operand::Place(p) => p.projection.iter().all(|pr| match pr {
            Projection::Deref | Projection::Downcast(_) => true,
            Projection::Field(_, t) => !matches!(t, Ty::Array(..) | Ty::Slice(_)),
            _ => false,
        }),
        ir::Operand::Aggregate(..) | ir::Operand::Fn(_) => false,
    }
}

/// A place a cast reads directly: a local, an element, through a pointer, or a one-word field (field forwarding
/// leaves a checked op's value there, `(_t.0: i16) as usize`).
fn castable(p: &ir::Place) -> bool {
    match p.projection.as_slice() {
        [Projection::Field(_, t)] => one_word(t),
        pr => pr
            .iter()
            .all(|pr| matches!(pr, Projection::Deref | Projection::Index(_) | Projection::IndexPlus(..))),
    }
}

/// A cast lowered as a conversion of one word: any kind but a pointer coercion or one the IR does not model.
fn converts(kind: &ir::CastKind) -> bool {
    !matches!(kind, ir::CastKind::Unsize(_) | ir::CastKind::Other(_))
}

impl<'a> Lowering<'a> {
    /// A whole array copied, `b = copy a`, or a callee's array result, `b = copy @f.3`: element by element.
    /// False when `dest` is no array.
    fn copy_array(&mut self, dest: &ir::Place, from: &ir::Place, src: &str) -> Result<bool, Fail> {
        if !dest.projection.is_empty() || !from.projection.is_empty() {
            return Ok(false);
        }
        let Loc::At(to) = self.place_any(dest)? else {
            return Ok(false);
        };
        let Some(n) = self.array_at(&to).map(|a| a.len as u32) else {
            return Ok(false);
        };
        let Loc::At(at) = self.place_any(from)? else {
            return Ok(false);
        };
        for k in self.kept(&to, n) {
            self.emit_c(Op::Cla, Cell::element(&at.sym, k), if k == 0 { src } else { "" });
            self.emit(Op::Sto, Cell::element(&to.sym, k));
        }
        Ok(true)
    }

    /// In main's opening blocks: a constant stored into a local nothing has read yet becomes the cell's initial value.
    fn initial_value(&mut self, s: &Statement) -> Result<bool, Fail> {
        let Statement::Assign(d, rv) = s else { return Ok(false) };
        let Some(x) = d.as_local().and_then(numbered) else {
            return Ok(false);
        };
        let scalar = |o: &ir::Operand| matches!(o, ir::Operand::Constant(c) if c.is_scalar());
        let shape = match rv {
            Rvalue::Use(o) | Rvalue::Repeat(o, _) => scalar(o),
            Rvalue::Array(items) => items.iter().all(scalar),
            _ => false,
        };
        let once = self.constant_arrays.contains(&x);
        if !shape || self.pooled.contains(&x) || (self.seen.contains(&x) && !once) {
            return Ok(false);
        }
        // main's entry block runs exactly once, before anything reads this local (or nothing else ever writes it):
        // make it the cell's initial value
        let c = self.cell(x, None);
        let whole = Cell::from(c.clone());
        match rv {
            Rvalue::Repeat(o, n) => {
                let v = self.operand(o)?.k();
                for k in self.kept(&whole, *n as u32) {
                    self.set_init(Cell::element(&c, k), v);
                }
            }
            Rvalue::Array(items) => {
                let kept = self.kept(&whole, items.len() as u32);
                for (k, it) in items.iter().enumerate().filter(|(k, _)| kept.contains(&(*k as u32))) {
                    let v = self.operand(it)?.k();
                    self.set_init(Cell::element(&c, k as u32), v);
                }
            }
            Rvalue::Use(o) => {
                let v = self.operand(o)?.k();
                self.set_init(Cell::from(c), v);
            }
            _ => unreachable!(),
        }
        self.seen.insert(x);
        Ok(true)
    }

    fn set_init(&mut self, cell: Cell, v: Option<i64>) {
        match v {
            Some(v) => {
                self.init.insert(cell, v);
            }
            None => {
                self.init.remove(&cell);
            }
        }
    }

    pub(super) fn statement(&mut self, s: &Statement) -> Result<(), Fail> {
        let src = source(s);
        let src = src.as_str();
        if !self.init_block && self.constant_arrays.iter().any(|x| s.def() == Some(*x)) && self.initial_value(s)? {
            return Ok(());
        }
        if self.init_block {
            if self.initial_value(s)? {
                return Ok(());
            }
            for t in s.locals() {
                if t != Local::UNIT {
                    self.seen.insert(t);
                }
            }
        }
        let Statement::Assign(dest, rv) = s else {
            fail!(unsupported_statement(s))
        };
        match rv {
            Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow), x, y) => {
                // the result into .0 and whether it left its type's range into .1
                let Some(d) = dest.as_local() else {
                    fail!(unsupported_place(dest))
                };
                let f0 = self.cell(d, Some(0));
                let f1 = self.cell(d, Some(1));
                let a = self.operand(x)?;
                let a = self.mat(a);
                let b = self.operand(y)?;
                let b = self.mat(b);
                let (bad, done) = (self.fresh("ov"), self.fresh("d"));
                let w = self.width.get(&d).copied();
                let plain = plain_op(*op);
                if w == Some(IntTy::U8) && plain == BinOp::Mul {
                    // the product wraps, as a u8's does, but past a card it can't be held to be wrapped
                    fail!(overflowing_mul_u8())
                }
                self.arith_acc(plain, a, b, w.map(|_| &bad), src)?;
                self.emit(Op::Sto, f0.clone());
                self.check_acc(w, &bad, plain)?;
                let z = self.konst(0);
                self.emit(Op::Cla, z);
                self.emit(Op::Sto, f1.clone());
                self.emit(Op::Jmp, done.clone());
                self.define(&bad);
                if w == Some(IntTy::U8) && plain != BinOp::Mul {
                    // a u8's sum or difference wraps, as Rust's does (`overflowing_add` reads it): -255..=510 is on
                    // a card
                    self.emit(Op::Cla, f0.clone());
                    self.wrap_u8(plain == BinOp::Sub);
                    self.emit(Op::Sto, f0);
                }
                let one = self.konst(1);
                self.emit(Op::Cla, one);
                self.emit(Op::Sto, f1);
                self.define(&done);
                Ok(())
            }
            Rvalue::Lin(lin) => {
                // added up in the accumulator (see lin_fold)
                let dst = self.place_any(dest)?;
                self.wide = self.past.contains(&self.at);
                let mut terms = vec![];
                for (k, o) in lin {
                    let v = self.operand(o)?;
                    let v = self.mat(v);
                    terms.push((*k, v));
                }
                terms.sort_by_key(|t| t.0 <= 0); // a positive term first, so it can be loaded
                let mut first = true;
                for (k, v) in &terms {
                    for _ in 0..k.abs() {
                        if first && *k > 0 {
                            self.load(v)?;
                            first = false;
                            continue;
                        }
                        if first {
                            let z = self.konst(0);
                            self.emit(Op::Cla, z);
                            first = false;
                        }
                        let x = self.arith(v)?;
                        self.emit(if *k > 0 { Op::Add } else { Op::Sub }, x);
                    }
                }
                if let Some(l) = self.code.last_mut() {
                    l.note(src);
                }
                self.store(&dst, "")
            }
            Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), x, y) => {
                // with overflow checks off (rustc -C overflow-checks=off): no check, the cell keeps what it keeps
                let dst = self.place_any(dest)?;
                let mut a = self.operand(x)?;
                let mut b = self.operand(y)?;
                // a sum's term read through an address comes first, straight into the accumulator: the other is
                // the cell the ADD names, and neither needs a scratch cell
                if *op != BinOp::Sub && matches!(b, Value::Computed(_)) && !matches!(a, Value::Computed(_)) {
                    std::mem::swap(&mut a, &mut b);
                }
                let b = self.mat(b);
                // (+ and - load the first as it is, through its address; the multiply routine takes a cell)
                let a = if *op == BinOp::Mul { self.mat(a) } else { a };
                self.arith_acc(*op, a, b, None, src)?;
                // past a card the cell keeps the last three digits, and the accumulator all of it
                self.wide = self.past.contains(&self.at);
                self.store(&dst, "")
            }
            Rvalue::BinaryOp(op @ (BinOp::AddWrapping | BinOp::SubWrapping | BinOp::MulWrapping), x, y) => {
                // a u8's + - or * that can leave 0..=255 (see ranges::mark_wrapping): the plain op, then mod 256; a
                // usize's - that can go below 0, the plain op
                let dst = self.place_any(dest)?;
                let a = self.operand(x)?;
                let a = self.mat(a);
                let b = self.operand(y)?;
                let b = self.mat(b);
                let plain = crate::compile::ranges::unwrapped(*op);
                let ty = self.place_ty(dest);
                if let (BinOp::Mul, Some(p), Some(q)) = (plain, a.k(), b.k()) {
                    // both known: the product wrapped as Rust wraps it, as rustc would have folded it
                    let v = if ty == Some(Ty::Int(IntTy::I8)) {
                        (p * q + 128).rem_euclid(256) - 128
                    } else {
                        (p * q).rem_euclid(256)
                    };
                    let v = self.konst(v);
                    self.emit_c(Op::Cla, v, src);
                    return self.store(&dst, "");
                }
                // a u8's or an i8's product by a constant to 39 is whole in the accumulator (9999 at most), and wraps
                // as Rust's does. Any other is the routine's, which stores its partial products: past a card the
                // product is what CARDIAC keeps of it, as 999 + 1 stores 000, its last three digits with its sign,
                // and the wrap takes those (by a constant, more of it would be left in the accumulator)
                let held = |v: &Value| v.k().is_some_and(|k| (-39..=39).contains(&k));
                if plain == BinOp::Mul && !(held(&a) || held(&b)) {
                    // the constant second: it is how many adds
                    let (a, b) = if a.k().is_some() { (b, a) } else { (a, b) };
                    self.mul_call(&a, &b, false, src)?;
                } else {
                    self.arith_acc(plain, a, b, None, src)?;
                }
                if ty == Some(Ty::Int(IntTy::I8)) {
                    self.wrap_i8();
                } else if ty == Some(Ty::Int(IntTy::Usize)) {
                    // a usize below 0 is one past a card, which only a local that can hold one does (as it is)
                    if !dest.as_local().is_some_and(|l| self.huge.contains(&l)) {
                        self.past_a_card("a usize past a card");
                    }
                } else {
                    self.wrap_u8(plain == BinOp::Sub);
                }
                self.store(&dst, "")
            }
            Rvalue::UnaryOp(UnOp::Neg, x) => {
                let dst = self.place_any(dest)?;
                let a = self.operand(x)?;
                let a = self.mat(a);
                let z = self.konst(0);
                self.emit_c(Op::Cla, z, src);
                let x = self.arith(&a)?;
                self.emit(Op::Sub, x);
                if self.place_ty(dest) == Some(Ty::Int(IntTy::I8)) {
                    // -(-128) wraps to -128 in an i8 (overflow checks off: on, rustc's assert comes first)
                    self.wrap_i8();
                }
                self.store(&dst, "")
            }
            Rvalue::UnaryOp(UnOp::Not, x) => {
                // bool: 1 - b (cell 00 holds 1)
                let dst = self.place_any(dest)?;
                let a = self.operand(x)?;
                let a = self.mat(a);
                // a local, an element or a field
                let ty = x.place().and_then(|p| self.place_ty(p));
                let is_bool = ty == Some(Ty::Bool) || matches!(x, ir::Operand::Constant(Const::Bool(_)));
                if !is_bool {
                    fail!(not_type(src))
                }
                self.emit_c(Op::Cla, Operand::CELL_00, src);
                let x = self.arith(&a)?;
                self.emit(Op::Sub, x);
                self.store(&dst, "")
            }
            Rvalue::BinaryOp(op @ (BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor), x, y)
                if [x, y].iter().any(|o| matches!(self.operand_ty(o), Some(Ty::Int(_)))) =>
            {
                // `x & m` for a mask m one below a power of two is x.rem_euclid(m + 1); anything else, the routine
                let mask = |o: &ir::Operand| o.int().map(|(k, _)| k).filter(|&k| k >= 0 && (k + 1) & k == 0);
                match (*op, mask(y), mask(x)) {
                    (BinOp::BitAnd, Some(m), _) => self.mask(src, dest, x, m as i64),
                    (BinOp::BitAnd, None, Some(m)) => self.mask(src, dest, y, m as i64),
                    _ => self.bitwise(src, dest, *op, x, y),
                }
            }
            Rvalue::BinaryOp(op @ (BinOp::Shl | BinOp::Shr), x, y) => {
                // by a constant on an i16 or a u8 they are arithmetic already (`methods::inline_operators`), up to 9
                // and 7; a routine for the rest
                self.shift(src, dest, *op == BinOp::Shl, x, y)
            }
            Rvalue::BinaryOp(op @ (BinOp::BitAnd | BinOp::BitOr), x, y) => {
                // bools only (integers are above): and = a * b, or = a + b - a b
                let dst = self.place_any(dest)?;
                let a = self.operand(x)?;
                let a = self.mat(a);
                let b = self.operand(y)?;
                let b = self.mat(b);
                let (one, done) = (self.fresh("b1"), self.fresh("d"));
                let and = *op == BinOp::BitAnd;
                // a is 0: the answer is b (or) or 0 (and)
                self.load(&a)?;
                self.emit_c(Op::Sub, Operand::CELL_00, src);
                self.emit(Op::Tac, one.clone());
                let x = if and { self.arith(&b)? } else { self.konst(1) };
                self.emit(Op::Cla, x);
                self.emit(Op::Jmp, done.clone());
                self.define(&one);
                let x = if and { self.konst(0) } else { self.arith(&b)? };
                self.emit(Op::Cla, x);
                self.define(&done);
                self.store(&dst, "")
            }
            Rvalue::BinaryOp(op @ (BinOp::URem | BinOp::UDiv), x, y) => {
                self.divide(src, dest, *op == BinOp::URem, x, y, true)
            }
            Rvalue::BinaryOp(op @ (BinOp::Rem | BinOp::Div), x, y) => {
                self.divide(src, dest, *op == BinOp::Rem, x, y, false)
            }
            Rvalue::BinaryOp(op, ir::Operand::Constant(ca), ir::Operand::Constant(cb))
                if is_comparison(*op) && ca.is_scalar() && cb.is_scalar() =>
            {
                // both sides known: fold
                let a = self.operand(&ir::Operand::Constant(ca.clone()))?.k();
                let b = self.operand(&ir::Operand::Constant(cb.clone()))?.k();
                let v = match op {
                    BinOp::Eq => a == b,
                    BinOp::Ne => a != b,
                    BinOp::Lt => a < b,
                    BinOp::Le => a <= b,
                    BinOp::Gt => a > b,
                    _ => a >= b,
                };
                let c = self.konst(if v { 1 } else { 0 });
                self.emit(Op::Cla, c);
                let p = self.place(dest)?;
                self.emit_c(Op::Sto, p, src);
                Ok(())
            }
            Rvalue::BinaryOp(op, x, y) if is_comparison(*op) => self.compare(*op, dest, x, y, src),
            Rvalue::Repeat(o, n) if repeatable(o) => {
                // [v; N]
                let base = self.place(dest)?;
                let n = *n as usize;
                if self.array_at(&base).map(|a| a.len) != Some(n) {
                    fail!(internal(format!("array repeat size mismatch: {src}")))
                }
                let v = self.operand(o)?;
                let v = self.mat(v);
                self.load(&v)?;
                let whole = Cell::from(base.sym.clone());
                for k in self.kept(&whole, n as u32) {
                    let at = Cell::element(&base.sym, k);
                    self.emit_c(Op::Sto, at, if k > 0 { "" } else { src });
                }
                Ok(())
            }
            Rvalue::Array(items) => {
                // array literal
                let base = self.place(dest)?;
                if self.array_at(&base).map(|a| a.len) != Some(items.len()) {
                    fail!(internal(format!("array literal size mismatch: {src}")))
                }
                let whole = Cell::from(base.sym.clone());
                for (k, it) in items.iter().enumerate() {
                    if self.hole_at(&whole, k) {
                        continue;
                    }
                    let v = self.operand(it)?;
                    let v = self.mat(v);
                    self.load(&v)?;
                    let at = Cell::element(&base.sym, k as u32);
                    self.emit_c(Op::Sto, at, if k > 0 { "" } else { src });
                }
                Ok(())
            }
            Rvalue::Ref(_, p) => {
                // taking an address
                let dst = self.place_any(dest)?;
                match self.place_any(p)? {
                    Loc::Computed(d) => {
                        self.run_dyn(&d, None);
                    }
                    Loc::At(c) => {
                        let a = self.addr(c);
                        self.emit(Op::Cla, a);
                    }
                }
                self.store(&dst, src)
            }
            Rvalue::Cast(ir::Operand::Copy(p) | ir::Operand::Move(p), to, kind)
                if castable(p) && one_word(to) && converts(kind) =>
            {
                self.cast(dest, p, to, src)
            }
            // a constant's cast, `WORDS as i16`: the value, known now, as the type has it
            Rvalue::Cast(c @ ir::Operand::Constant(_), Ty::Int(to), kind) if converts(kind) => {
                let Some(v) = self.operand(c)?.k() else {
                    fail!(unsupported_place(c))
                };
                let fits = match to {
                    IntTy::U8 => (0..=255).contains(&v),
                    IntTy::I8 => (-128..=127).contains(&v),
                    IntTy::Usize => (0..=999).contains(&v),
                    _ => (-999..=999).contains(&v),
                };
                if !fits {
                    fail!(cast_past_card(v, to, src))
                }
                let dst = self.place_any(dest)?;
                let k = self.konst(v);
                self.emit_c(Op::Cla, k, src);
                self.store(&dst, "")
            }
            Rvalue::Use(ir::Operand::Copy(p) | ir::Operand::Move(p)) if self.copy_array(dest, p, src)? => Ok(()),
            _ => {
                let dst = self.place_any(dest)?;
                let v = match rv {
                    Rvalue::Use(o) => self.operand(o)?,
                    // anything else is no operand this backend knows
                    other => {
                        let text = other.to_string();
                        let text = text
                            .strip_prefix("copy ")
                            .or_else(|| text.strip_prefix("move "))
                            .unwrap_or(&text);
                        fail!(unsupported_place(text))
                    }
                };
                let carried = matches!(&v, Value::At(c) if self.carried.as_ref() == Some(c));
                if let Loc::Computed(d) = &dst {
                    if !matches!(v, Value::Computed(_)) && !carried {
                        // xs[i] = v: build the STO on the element's address first, then load v and let it run
                        let f = self.run_dyn(d, Some(Access::Write));
                        let c = self.selfmod(Access::Write, "write through address", f);
                        self.load(&v)?;
                        self.cell_here(&c);
                        let at = self.code.len() - 2;
                        self.code[at].note(src);
                        return Ok(());
                    }
                }
                self.load(&v)?;
                self.store(&dst, src)
            }
        }
    }

    /// A comparison into a bool: 1 when it holds, 0 when not.
    fn compare(
        &mut self,
        op: BinOp,
        dest: &ir::Place,
        x: &ir::Operand,
        y: &ir::Operand,
        src: &str,
    ) -> Result<(), Fail> {
        let dst = self.place(dest)?;
        let a = self.operand(x)?;
        let mut a = self.mat(a);
        let b = self.operand(y)?;
        let mut b = self.mat(b);
        let (t, f, done) = (self.fresh("t"), self.fresh("f"), self.fresh("d"));
        self.huge_order(op, x, y, &t, &f)?;
        let mut op = op;
        if op == BinOp::Gt {
            std::mem::swap(&mut a, &mut b);
            op = BinOp::Lt;
        }
        if op == BinOp::Ge {
            std::mem::swap(&mut a, &mut b);
            op = BinOp::Le;
        }
        // a < b  <=>  a - b < 0 ;  a == b  <=>  !(a - b < 0) && !(b - a < 0)
        if op == BinOp::Lt {
            self.load(&a)?;
            let x = self.arith(&b)?;
            self.emit_c(Op::Sub, x, src);
            self.emit(Op::Tac, t.clone());
        } else if op == BinOp::Le {
            self.load(&b)?;
            let x = self.arith(&a)?;
            self.emit_c(Op::Sub, x, src);
            self.emit(Op::Tac, f.clone());
            self.emit(Op::Jmp, t.clone());
        } else {
            let (yes, no) = if op == BinOp::Eq { (&t, &f) } else { (&f, &t) };
            self.load(&a)?;
            let x = self.arith(&b)?;
            self.emit_c(Op::Sub, x, src);
            self.emit(Op::Tac, no.clone());
            self.load(&b)?;
            let x = self.arith(&a)?;
            self.emit(Op::Sub, x);
            self.emit(Op::Tac, no.clone());
            self.emit(Op::Jmp, yes.clone());
        }
        self.define(&f);
        let z = self.konst(0);
        self.emit(Op::Cla, z);
        self.emit(Op::Sto, dst.clone());
        self.emit(Op::Jmp, done.clone());
        self.define(&t);
        let one = self.konst(1);
        self.emit(Op::Cla, one);
        self.emit(Op::Sto, dst);
        self.define(&done);
        Ok(())
    }

    /// `p as T` between the integer types.
    fn cast(&mut self, dest: &ir::Place, p: &ir::Place, to: &Ty, src: &str) -> Result<(), Fail> {
        let dst = self.place_any(dest)?;
        let int = |t: Option<&Ty>| match t {
            Some(Ty::Int(i)) => Some(*i),
            _ => None,
        };
        // the type of what is cast: a local's, a field's, an element's, what a reference points to
        let from = match p.projection.as_slice() {
            [Projection::Field(_, t)] => int(Some(t)),
            _ => int(self.place_ty(p).as_ref()),
        };
        let to = int(Some(to));
        let v = self.operand(&ir::Operand::Place(p.clone()))?;
        self.load(&v)?;
        let huge = self.is_huge(&ir::Operand::Copy(p.clone()));
        if to == Some(IntTy::U8) && matches!(from, Some(IntTy::Usize | IntTy::I16)) {
            // `as u8` keeps the value mod 256, as Rust does: a negative i16 is raised by 256s first, and so is a
            // usize past a card, 2^64 - k held as -k, which has -k's (where the range pass knows the value is
            // 0..=255 already, it made the cast a copy)
            if huge || from == Some(IntTy::I16) {
                let (up, back, ok) = (self.fresh("cast"), self.fresh("cast"), self.fresh("d"));
                let k = self.konst(256);
                self.define(&up);
                self.emit(Op::Tac, back.clone());
                self.emit(Op::Jmp, ok.clone());
                self.define(&back);
                self.emit(Op::Add, k);
                self.emit(Op::Jmp, up);
                self.define(&ok);
            }
            let (lp, done) = (self.fresh("cast"), self.fresh("d"));
            self.define(&lp);
            let k = self.konst(256);
            self.emit(Op::Sub, k.clone());
            self.emit(Op::Tac, done.clone());
            self.emit(Op::Jmp, lp);
            self.define(&done);
            self.emit(Op::Add, k);
        } else if to == Some(IntTy::I8) && from != Some(IntTy::I8) {
            // `as i8` keeps the value mod 256, as Rust does, from -128 to 127: a usize past a card is held below 0,
            // as the value it is mod 256
            self.wrap_i8();
        } else if from == Some(IntTy::I8) && to == Some(IntTy::U8) {
            // a negative i8 is 256 more as a u8
            let (up, done) = (self.fresh("cast"), self.fresh("d"));
            let k = self.konst(256);
            self.emit(Op::Tac, up.clone());
            self.emit(Op::Jmp, done.clone());
            self.define(&up);
            self.emit(Op::Add, k);
            self.define(&done);
        } else if to == Some(IntTy::Usize) && (matches!(from, Some(IntTy::I16 | IntTy::I8)) || huge) {
            // a negative i16 is a usize past a card in Rust (-1 is usize::MAX): held below 0 as it is, in a local
            // that can hold one, else past a card (see ranges::huge_usizes)
            if !dest.as_local().is_some_and(|l| self.huge.contains(&l)) {
                self.past_a_card("as usize: a usize past a card");
            }
        } else if !matches!(to, Some(IntTy::Usize | IntTy::U8 | IntTy::I8 | IntTy::I16)) {
            fail!(unsupported_cast(src))
        }
        self.store(&dst, src)
    }
}

/// A usize past a card (see `ranges::huge_usizes`) can be held as the negative number it is: these.
impl<'a> Lowering<'a> {
    /// Whether an operand reads a whole local that can hold a usize past a card.
    pub(super) fn is_huge(&self, o: &ir::Operand) -> bool {
        o.local()
            .is_some_and(|l| self.huge.contains(&l) && !self.on_card_now.contains(&l))
    }

    /// A branch to the panic routine when the accumulator is below 0: a usize past a card that goes where only one
    /// a card holds can, the check of an op on a card that would overflow.
    fn past_a_card(&mut self, comment: &str) {
        let p = self.use_panic();
        let check = self.panic.replace(NO_CARD.into());
        self.emit_c(Op::Tac, p, comment);
        self.panic = check;
    }

    /// Before a statement that reads a usize past a card where a card can't hold it (anything but a copy, sum or
    /// difference into another such local, a cast, a comparison, or an index): the panic if it is one.
    pub(super) fn guard_huge(&mut self, s: &Statement) {
        let Statement::Assign(d, rv) = s else { return };
        let into_huge = d.as_local().is_some_and(|l| self.huge.contains(&l));
        let held = match rv {
            Rvalue::Cast(..) => true,
            Rvalue::BinaryOp(op, ..) if is_comparison(*op) => true,
            Rvalue::Use(_)
            | Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::AddWrapping | BinOp::SubWrapping, ..) => into_huge,
            _ => false,
        };
        if held {
            return;
        }
        let read: Vec<Local> = rv
            .operands()
            .into_iter()
            .filter(|o| self.is_huge(o))
            .filter_map(|o| o.local())
            .collect();
        self.guard_each(read);
    }

    /// Before a call: the panic if an argument is a usize past a card (the callee's parameter holds one a card holds).
    pub(super) fn guard_huge_args(&mut self, t: &ir::Terminator) {
        if let ir::Terminator::Call { args, .. } = t {
            let read: Vec<Local> = args
                .iter()
                .filter(|o| self.is_huge(o))
                .filter_map(|o| o.local())
                .collect();
            self.guard_each(read);
        }
    }

    fn guard_each(&mut self, mut read: Vec<Local>) {
        read.sort_by_key(|l| l.0);
        read.dedup();
        for l in read {
            let c = self.cell(l, None);
            self.emit_c(Op::Cla, c, &format!("{l}: a usize past a card?"));
            self.past_a_card("");
        }
    }

    /// Before a usize comparison one of whose sides can be past a card, held below 0: the sign tests that settle it
    /// when only one side is, since such a usize is above every usize a card holds. Past them both sides are on one
    /// side of 0, where the comparison of the numbers held is the comparison of the usizes. `t` is where the
    /// comparison holds, `f` where it doesn't.
    pub(super) fn huge_order(
        &mut self,
        op: BinOp,
        x: &ir::Operand,
        y: &ir::Operand,
        t: &Sym,
        f: &Sym,
    ) -> Result<(), Fail> {
        let (wa, wb) = (self.is_huge(x), self.is_huge(y));
        let (less, greater) = match op {
            BinOp::Lt | BinOp::Le => (t, f),
            BinOp::Gt | BinOp::Ge => (f, t),
            _ => return Ok(()),
        };
        let (less, greater) = (less.clone(), greater.clone());
        if wa && wb {
            let (below, same) = (self.fresh("past"), self.fresh("same"));
            let a = self.operand(x)?;
            self.load(&a)?;
            self.emit_c(Op::Tac, below.clone(), "a usize past a card is above the rest");
            let b = self.operand(y)?;
            self.load(&b)?;
            self.emit(Op::Tac, less);
            self.emit(Op::Jmp, same.clone());
            self.define(&below);
            let b = self.operand(y)?;
            self.load(&b)?;
            self.emit(Op::Tac, same.clone());
            self.emit(Op::Jmp, greater);
            self.define(&same);
        } else if wa {
            let a = self.operand(x)?;
            self.load(&a)?;
            self.emit_c(Op::Tac, greater, "a usize past a card is above the rest");
        } else if wb {
            let b = self.operand(y)?;
            self.load(&b)?;
            self.emit_c(Op::Tac, less, "a usize past a card is above the rest");
        }
        Ok(())
    }
}
