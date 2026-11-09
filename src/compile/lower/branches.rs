//! Terminators that branch: gotos, returns, switches and asserts; compare-and-branch; and the fusing of a
//! comparison or an overflow-checked op into the branch that reads its bool, which then is never stored.
use crate::compile::ir::{
    self, blocks_naming, BasicBlock, BinOp, IntTy, Local, Projection, Rvalue, Statement, Terminator, Ty,
};
use crate::compile::machine::{Arg, Op};
use crate::compile::Fail;

use super::arith::plain_op;
use super::places::{Loc, Value};
use super::{bare, is_comparison, numbered, one_word, plain_unwind, source, Cell, Lowering, Sym};

/// How many lines of a block read the value of an overflow-checked pair `(t.0: T)` (with `skip_asserts`, not
/// counting an assert's message arguments).
fn value_reads(b: &ir::Block, t: Local, skip_asserts: bool) -> usize {
    let s = b
        .statements
        .iter()
        .filter(|s| s.places().iter().any(|p| p.is_value_of(t)))
        .count();
    let term = !(skip_asserts && matches!(b.terminator, Terminator::Assert { .. }))
        && b.terminator.places().iter().any(|p| p.is_value_of(t));
    s + term as usize
}

/// A switch arm's value as a number (a value too large for one saturates).
fn clamp(v: i128) -> i64 {
    v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

impl<'a> Lowering<'a> {
    pub(super) fn terminator(&mut self, t: &Terminator) -> Result<(), Fail> {
        match t {
            Terminator::Goto(b) => {
                let l = self.label(*b);
                self.emit_c(Op::Jmp, l, "goto");
                Ok(())
            }
            Terminator::Return => {
                if self.func == "main" {
                    self.emit_c(Op::Hrs, Arg::Num("0".into()), "return from main")
                } else {
                    let e = Sym::Exit(self.func.clone());
                    self.emit_c(Op::Jmp, e, "return")
                }
                Ok(())
            }
            Terminator::SwitchInt { discr, arms, otherwise } => {
                // an element or a field has its type too (a local's own cell is found by `signed`)
                let ty = discr.place().and_then(|p| self.place_ty(p));
                self.unsigned_switch = matches!(ty, Some(Ty::Bool | Ty::Int(IntTy::U8)));
                if let (Some(Ty::Bool), Some(l)) = (&ty, discr.place().and_then(|p| p.as_local())) {
                    let name = self.names.iter().find(|(_, x)| **x == l).map(|(n, _)| n.clone());
                    self.sh.bool_tests.push(name);
                }
                let v = self.operand(discr)?;
                let vv = self.mat(v);
                // arms on consecutive values v0, v0 + 1, ... (a match's `0 => .., 1 => .., 2 => ..`): one subtraction
                // for them all, x - v0 below 0 the other way, then each SUB 00 and TAC the next arm: two cells an
                // arm, not an equality's two tests
                let mut sorted: Vec<(i64, BasicBlock)> = arms.iter().map(|(v, t)| (clamp(*v), *t)).collect();
                sorted.sort();
                let consecutive = sorted.windows(2).all(|w| w[1].0 == w[0].0 + 1);
                if let (true, true, Some(other)) = (sorted.len() >= 2, consecutive, otherwise) {
                    let (v0, other) = (sorted[0].0, self.label(*other));
                    if (-999..=999).contains(&v0) {
                        self.load(&vv)?;
                        let comment = format!("switchInt == {v0} to {}", v0 + sorted.len() as i64 - 1);
                        if v0 != 0 {
                            let k = self.konst(v0);
                            self.emit_c(Op::Sub, k, &comment);
                            self.emit(Op::Tac, other.clone());
                        } else {
                            self.emit_c(Op::Tac, other.clone(), &comment);
                        }
                        for (_, target) in &sorted {
                            let one = self.konst(1);
                            self.emit(Op::Sub, one);
                            let t = self.label(*target);
                            self.emit(Op::Tac, t);
                        }
                        self.unsigned_switch = false;
                        self.emit(Op::Jmp, other);
                        return Ok(());
                    }
                }
                for (value, target) in arms {
                    let val = clamp(*value);
                    let next = self.fresh("n");
                    let target = self.label(*target);
                    // a - c >= 0 and a - c - 1 < 0
                    self.branch_on(
                        BinOp::Eq,
                        vv.clone(),
                        Value::Const(val),
                        &target,
                        &next,
                        &format!("switchInt == {val}"),
                    )?;
                    self.define(&next);
                }
                self.unsigned_switch = false;
                let Some(other) = otherwise else {
                    fail!(internal(format!("switchInt without otherwise: {t}")))
                };
                let l = self.label(*other);
                self.emit(Op::Jmp, l);
                Ok(())
            }
            Terminator::Assert {
                cond,
                expected,
                target,
                unwind,
                ..
            } if plain_unwind(unwind) => {
                let cond_place = cond.place();
                let cond_text = bare(cond);
                let negated = !*expected;
                let flag = cond_place.and_then(|p| match p.projection.as_slice() {
                    [Projection::Field(1, Ty::Bool)] => p.base_local().and_then(numbered),
                    _ => None,
                });
                if negated {
                    if let Some(f) = flag.filter(|f| self.fused_flags.contains(f)) {
                        // The op that set this flag was checked when it ran, and every path here went through that
                        // check.
                        let l = self.label(*target);
                        self.emit_c(Op::Jmp, l, &format!("{f} already checked"));
                        return Ok(());
                    }
                }
                // a check worked out already (blocks::settled_checks): a goto, or the panic
                if let ir::Operand::Constant(ir::Const::Bool(holds)) = cond {
                    if *holds == *expected {
                        let l = self.label(*target);
                        self.emit_c(Op::Jmp, l, &format!("assert({cond_text}) holds"));
                    } else {
                        let fail = self.use_panic();
                        let c = format!("assert({}{cond_text}) fails", if negated { "!" } else { "" });
                        self.emit_c(Op::Jmp, fail, &c);
                    }
                    return Ok(());
                }
                let cond = match cond_place {
                    Some(p) => self.place(p)?,
                    None => fail!(unsupported_place(cond_text)),
                };
                let fail = self.use_panic();
                // success when cond is false (if negated) or true; anything else panics
                self.emit_c(
                    Op::Cla,
                    cond,
                    &format!("assert({}{cond_text})", if negated { "!" } else { "" }),
                );
                let one = self.konst(1);
                self.emit(Op::Sub, one);
                let l = self.label(*target);
                if negated {
                    self.emit(Op::Tac, l);
                    self.emit(Op::Jmp, fail);
                } else {
                    self.emit(Op::Tac, fail);
                    self.emit(Op::Jmp, l);
                }
                self.use_panic();
                Ok(())
            }
            _ => self.call(t),
        }
    }

    /// Compare-and-branch: jump to T when the comparison holds, F otherwise. u8 and bool are never negative.
    pub(super) fn branch_on(
        &mut self,
        op: BinOp,
        a: Value,
        b: Value,
        t: &Sym,
        f: &Sym,
        comment: &str,
    ) -> Result<(), Fail> {
        if let Some(done) = self.branch_through_accumulator(op, &a, &b, t, f, comment)? {
            return Ok(done);
        }
        let (a, b) = (self.mat(a), self.mat(b));
        // against a constant, always load the other side first (so it can come carried in the accumulator)
        let flip = |op: BinOp| match op {
            BinOp::Lt => BinOp::Gt,
            BinOp::Gt => BinOp::Lt,
            BinOp::Le => BinOp::Ge,
            BinOp::Ge => BinOp::Le,
            x => x,
        };
        if a.k().is_some() && b.k().is_none() {
            return self.branch_on(flip(op), b, a, t, f, comment);
        }
        if let (Some(c), None) = (b.k(), a.k()) {
            if (op == BinOp::Gt || op == BinOp::Le) && (c + 1).abs() <= 999 {
                // a > c  <=>  a - (c + 1) is not negative;  a <= c  <=>  a - (c + 1) < 0
                self.load(&a)?;
                let k = self.konst(c + 1);
                self.emit_c(Op::Sub, k, comment);
                if op == BinOp::Le {
                    self.emit(Op::Tac, t.clone());
                    self.emit(Op::Jmp, f.clone());
                } else {
                    self.emit(Op::Tac, f.clone());
                    self.emit(Op::Jmp, t.clone());
                }
                return Ok(());
            }
            if op == BinOp::Ge {
                self.load(&a)?;
                let x = self.arith(&b)?;
                self.emit_c(Op::Sub, x, comment);
                self.emit(Op::Tac, f.clone());
                self.emit(Op::Jmp, t.clone());
                return Ok(());
            }
        }
        match op {
            // a > b  <=>  b - a < 0, here rather than as b < a: with b a constant (999, where a > c can't be a test
            // of c + 1) that would come straight back
            BinOp::Gt => {
                self.load(&b)?;
                let x = self.arith(&a)?;
                self.emit_c(Op::Sub, x, comment);
                self.emit(Op::Tac, t.clone());
                self.emit(Op::Jmp, f.clone());
                return Ok(());
            }
            BinOp::Ge => return self.branch_on(BinOp::Le, b, a, t, f, comment),
            BinOp::Ne => return self.branch_on(BinOp::Eq, a, b, f, t, comment),
            BinOp::Lt => {
                self.load(&a)?;
                let x = self.arith(&b)?;
                self.emit_c(Op::Sub, x, comment);
                self.emit(Op::Tac, t.clone());
                self.emit(Op::Jmp, f.clone());
                return Ok(());
            }
            BinOp::Le => {
                self.load(&b)?;
                let x = self.arith(&a)?;
                self.emit_c(Op::Sub, x, comment);
                self.emit(Op::Tac, f.clone());
                self.emit(Op::Jmp, t.clone());
                return Ok(());
            }
            _ => {}
        }
        let (a, b) = if b.k().is_none() && a.k().is_some() {
            (b, a)
        } else {
            (a, b)
        };
        if let Some(c) = b.k() {
            // a == c  <=>  a - c >= 0  and  a - c - 1 < 0
            self.load(&a)?;
            if c != 0 {
                let k = self.konst(c);
                self.emit_c(Op::Sub, k, comment);
                self.emit(Op::Tac, f.clone());
                let one = self.konst(1);
                self.emit(Op::Sub, one);
            } else {
                // == 0: a - 1 < 0, and for a signed value also not negative
                if self.signed(&a) {
                    self.emit_c(Op::Tac, f.clone(), comment);
                }
                let one = self.konst(1);
                let signed = self.signed(&a);
                self.emit_c(Op::Sub, one, if signed { "" } else { comment });
            }
            self.emit(Op::Tac, t.clone());
            self.emit(Op::Jmp, f.clone());
            return Ok(());
        }
        self.load(&a)?;
        let x = self.arith(&b)?;
        self.emit_c(Op::Sub, x, comment);
        self.emit(Op::Tac, f.clone());
        self.load(&b)?;
        let x = self.arith(&a)?;
        self.emit(Op::Sub, x);
        self.emit(Op::Tac, f.clone());
        self.emit(Op::Jmp, t.clone());
        Ok(())
    }

    /// An ordering comparison (<, <=, >, >=) of two values neither of them a constant, when one comes through the
    /// accumulator (an array element, read through its address). CARDIAC subtracts only a cell, so that one is
    /// loaded and the other subtracted from it: the comparison mirrored if need be, `x < y` as `y - x - 1` not
    /// negative, one SUB of 1 instead of parking it in a scratch cell and loading the other (store, load, and the
    /// cell). When both do, the one subtracted is parked. None when this doesn't apply, and nothing is emitted.
    fn branch_through_accumulator(
        &mut self,
        op: BinOp,
        a: &Value,
        b: &Value,
        t: &Sym,
        f: &Sym,
        comment: &str,
    ) -> Result<Option<()>, Fail> {
        let through = |value: &Value| matches!(value, Value::Computed(_));
        let carried = |value: &Value| matches!(value, Value::At(cell) if self.carried.as_ref() == Some(cell));
        if a.k().is_some() || b.k().is_some() || !(through(a) || through(b)) || carried(a) || carried(b) {
            return Ok(None);
        }
        // as x < y or x <= y; the default forms load x for <, and y for <=
        let (strict, x, y) = match op {
            BinOp::Lt => (true, a, b),
            BinOp::Gt => (true, b, a),
            BinOp::Le => (false, a, b),
            BinOp::Ge => (false, b, a),
            _ => return Ok(None),
        };
        let (loaded, subtracted) = if strict { (x, y) } else { (y, x) };
        if through(loaded) {
            // the default form already loads it: park the other only if it comes through the accumulator too
            let subtracted = self.mat(subtracted.clone());
            self.load(loaded)?;
            let cell = self.arith(&subtracted)?;
            self.emit_c(Op::Sub, cell, comment);
            if strict {
                // x - y < 0
                self.emit(Op::Tac, t.clone());
                self.emit(Op::Jmp, f.clone());
            } else {
                // y - x < 0: not x <= y
                self.emit(Op::Tac, f.clone());
                self.emit(Op::Jmp, t.clone());
            }
            return Ok(Some(()));
        }
        // mirrored: load the one that comes through the accumulator, subtract the one in its cell, then 1
        self.load(subtracted)?;
        let cell = self.arith(loaded)?;
        self.emit_c(Op::Sub, cell, comment);
        let one = self.konst(1);
        self.emit(Op::Sub, one);
        if strict {
            // y - x - 1 < 0 is y <= x: not x < y
            self.emit(Op::Tac, f.clone());
            self.emit(Op::Jmp, t.clone());
        } else {
            // x - y - 1 < 0 is x < y + 1: x <= y
            self.emit(Op::Tac, t.clone());
            self.emit(Op::Jmp, f.clone());
        }
        Ok(Some(()))
    }

    /// A comparison feeding a switchInt, or an overflow-checked op feeding its assert, as one branch: the bool is
    /// never stored. With `check_only`, only whether it can be done.
    pub(super) fn fuse(&mut self, stmt: &Statement, term: &Terminator, check_only: bool) -> Result<bool, Fail> {
        let s = source(stmt);
        let s = s.as_str();
        let Statement::Assign(dest, rv) = stmt else {
            return Ok(false);
        };
        let Some(m1) = dest.as_local().and_then(numbered) else {
            return Ok(false);
        };
        let reads = |o: &ir::Operand, x: Local| {
            matches!(o, ir::Operand::Copy(_) | ir::Operand::Move(_)) && o.local() == Some(x)
        };
        let moved = |o: &ir::Operand, x: Local| matches!(o, ir::Operand::Move(_)) && o.local() == Some(x);
        if let Rvalue::BinaryOp(op, x, y) = rv {
            if is_comparison(*op) {
                let op = *op;
                if let Terminator::SwitchInt {
                    discr,
                    arms,
                    otherwise: Some(yes),
                } = term
                {
                    if let (true, [(0, no)]) = (moved(discr, m1), arms.as_slice()) {
                        if check_only {
                            return Ok(true);
                        }
                        let (yes, no) = (self.label(*yes), self.label(*no));
                        self.huge_order(op, x, y, &yes, &no)?;
                        let a = self.operand(x)?;
                        let b = self.operand(y)?;
                        self.branch_on(op, a, b, &yes, &no, &format!("if {}({x}, {y})", op.name()))?;
                        return Ok(true);
                    }
                }
                let both_known =
                    matches!((x, y), (ir::Operand::Constant(c), ir::Operand::Constant(_)) if c.is_scalar());
                if !both_known {
                    if let Terminator::Assert {
                        cond,
                        expected: true,
                        target,
                        unwind,
                        ..
                    } = term
                    {
                        // (a copy too, if nothing else reads the bool: rustc's GVN asserts one r < 6 for each b[r][..],
                        // and all but the first are gone, `blocks::repeated_checks`)
                        if (moved(cond, m1) || reads(cond, m1) && self.never_carried(m1)) && plain_unwind(unwind) {
                            if check_only {
                                return Ok(true); // a bounds check: branch straight to success, otherwise panic
                            }
                            let ok = self.label(*target);
                            let fail = self.use_panic();
                            self.huge_order(op, x, y, &ok, &fail)?;
                            let a = self.operand(x)?;
                            let b = self.operand(y)?;
                            self.branch_on(op, a, b, &ok, &fail, &format!("assert {}({x}, {y})", op.name()))?;
                            return Ok(true);
                        }
                    }
                }
                if !both_known {
                    if let Terminator::Assert {
                        cond,
                        expected: false,
                        target,
                        unwind,
                        ..
                    } = term
                    {
                        // only if nothing else reads the bool: a fused check never stores it (rustc reuses a divide's
                        // zero check for the remainder after it)
                        if reads(cond, m1) && plain_unwind(unwind) && self.never_carried(m1) {
                            if check_only {
                                return Ok(true); // a check that must not hold (divide by zero, say): holds -> panic
                            }
                            let fail = self.use_panic();
                            let ok = self.label(*target);
                            self.huge_order(op, x, y, &fail, &ok)?;
                            let a = self.operand(x)?;
                            let b = self.operand(y)?;
                            self.branch_on(op, a, b, &fail, &ok, &format!("assert !{}({x}, {y})", op.name()))?;
                            return Ok(true);
                        }
                    }
                }
                if let (BinOp::Eq | BinOp::Ne, ir::Operand::Constant(ca), ir::Operand::Constant(cb)) = (op, x, y) {
                    if let Terminator::Assert {
                        cond,
                        expected: false,
                        target,
                        unwind,
                        ..
                    } = term
                    {
                        if ca.is_scalar() && cb.is_scalar() && moved(cond, m1) && plain_unwind(unwind) {
                            let a = self.operand(x)?.k();
                            let b = self.operand(y)?.k();
                            let holds = if op == BinOp::Eq { a == b } else { a != b };
                            if holds {
                                return Ok(false); // a check that would fail stays as code, and panics
                            }
                            if check_only {
                                return Ok(true);
                            }
                            let l = self.label(*target);
                            self.emit_c(Op::Jmp, l, &format!("{s}: known false, no check needed"));
                            return Ok(true);
                        }
                    }
                }
            }
        }
        let overflow = |t: &Terminator| -> Option<BasicBlock> {
            let Terminator::Assert {
                cond: ir::Operand::Copy(p) | ir::Operand::Move(p),
                expected: false,
                target,
                unwind,
                ..
            } = t
            else {
                return None;
            };
            if !plain_unwind(unwind)
                || p.base_local() != Some(m1)
                || !matches!(p.projection.as_slice(), [Projection::Field(1, Ty::Bool)])
            {
                return None;
            }
            Some(*target)
        };
        if let (Rvalue::Sum(terms), Some(succ)) = (rv, overflow(term)) {
            if check_only {
                return Ok(true);
            }
            self.fused_flags.insert(m1);
            let dest = self.direct_dest(m1, succ)?;
            self.use_panic();
            self.sum_acc(terms, s)?;
            self.emit(Op::Sto, dest);
            self.sh.pool.chk = true;
            self.emit_c(Op::Jmp, Sym::Runtime("rt_chk_entry"), "is the total still a card?");
            let r = self.fresh("rr");
            self.define(&r);
            let l = self.label(succ);
            self.emit(Op::Jmp, l);
            return Ok(true);
        }
        if let (
            Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow), x, y),
            Some(succ),
        ) = (rv, overflow(term))
        {
            if check_only {
                return Ok(true);
            }
            self.fused_flags.insert(m1); // later asserts of this flag are already known to pass
            let a = self.operand(x)?;
            let a = self.mat(a);
            let b = self.operand(y)?;
            let b = self.mat(b);
            let ok = self.label(succ);
            let fail = self.use_panic();
            // If the success block starts by moving the result out and nothing else jumps there, store straight
            // into the final place. On overflow we panic, so the early store is never observed.
            let mut dest = Cell::from(self.cell(m1, Some(0)));
            let uses: usize = self.blocks.iter().map(|b| value_reads(b, m1, false)).sum();
            if let Some(d) = self.move_out(m1, succ, uses)? {
                dest = d;
            }
            let ty = self.width.get(&m1).copied();
            let plain = plain_op(*op);
            let sign = match plain {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                _ => "*",
            };
            self.arith_acc(
                plain,
                a,
                b,
                ty.map(|_| &fail),
                &format!("{x} {sign} {y}, overflow-checked"),
            )?;
            self.emit(Op::Sto, dest);
            if ty == Some(IntTy::I16) {
                self.sh.pool.chk = true;
                self.use_panic();
                self.emit_c(Op::Jmp, Sym::Runtime("rt_chk_entry"), "is it still a card?");
                let r = self.fresh("rr");
                self.define(&r);
            } else {
                self.check_acc(ty, &fail, plain)?;
            }
            self.emit(Op::Jmp, ok);
            return Ok(true);
        }
        Ok(false)
    }

    /// When the success block starts by moving the op's result (t.0) out of its pair, nothing else reads it and only
    /// this block jumps there: the place to store the result in directly (the move becomes a nop). Never a pooled
    /// temporary: the pool hands out cells per block, so the store and the read would land in different cells.
    fn move_out(&mut self, t: Local, succ: BasicBlock, uses: usize) -> Result<Option<Cell>, Fail> {
        let Some(si) = self.blocks.iter().position(|x| x.name == succ) else {
            return Ok(None);
        };
        let Some(Statement::Assign(mv, Rvalue::Use(ir::Operand::Copy(p) | ir::Operand::Move(p)))) =
            self.blocks[si].statements.first()
        else {
            return Ok(None);
        };
        if !(p.base_local() == Some(t) && matches!(p.projection.as_slice(), [Projection::Field(0, ty)] if one_word(ty)))
        {
            return Ok(None);
        }
        let mv = mv.clone();
        // the local the destination starts with, when it is printed first: `_5`, `_5[_3]`
        let base = mv.base_local().filter(|_| mv.wrapped() == 0).and_then(numbered);
        if uses == 1
            && blocks_naming(&self.blocks, succ) == 1
            && !base.is_some_and(|b| self.pooled.contains(&b))
            && !self.emitted(succ)
        {
            if let Loc::At(c) = self.place_any(&mv)? {
                self.blocks[si].statements[0] = Statement::Nop;
                self.moved_out.extend(mv.base_local());
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    /// Where an op whose result is moved out at the start of its success block can store straight away.
    fn direct_dest(&mut self, t: Local, succ: BasicBlock) -> Result<Cell, Fail> {
        let dest = Cell::from(self.cell(t, Some(0)));
        let uses: usize = self.blocks.iter().map(|b| value_reads(b, t, true)).sum();
        Ok(self.move_out(t, succ, uses)?.unwrap_or(dest))
    }

    /// Could this operand be negative? (an i16 local, or a cell we cannot tell)
    fn signed(&mut self, op: &Value) -> bool {
        if let Some(k) = op.k() {
            return k < 0;
        }
        if self.unsigned_switch {
            return false;
        }
        let at = match op {
            Value::At(c) => Some(c.clone()),
            _ => None,
        };
        // which local's cell it is (this hands a pooled local, met on the way, its scratch cell); a usize that can be
        // past a card is held below 0, and a cell that one shares can be negative too
        let locals: Vec<Local> = self.locals.keys().copied().collect();
        let mut local = None;
        let mut huge = false;
        for l in locals {
            if Some(Cell::from(self.cell(l, None))) == at {
                local = local.or(Some(l));
                huge |= self.huge.contains(&l);
            }
        }
        match local {
            None => true,
            Some(l) => huge || !matches!(self.locals.get(&l), Some(Ty::Int(IntTy::U8 | IntTy::Usize) | Ty::Bool)),
        }
    }
}
