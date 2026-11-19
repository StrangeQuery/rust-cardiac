//! Terminators that branch: gotos, returns, switches and asserts; compare-and-branch; and the fusing of a
//! comparison or an overflow-checked op into the branch that reads its bool, which then is never stored.
use std::collections::HashSet;

use crate::compile::ir::{
    self, blocks_naming, BasicBlock, BinOp, IntTy, Local, Projection, Rvalue, Statement, Terminator, Ty,
};
use crate::compile::machine::{Arg, Kind, Line, Op};
use crate::compile::Fail;

use super::arith::plain_op;
use super::places::{Loc, Value};
use super::symbols::Access;
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

/// The values a dense switch sends to one block: `first..=last`, unbounded where none.
#[derive(Clone, Copy, Debug)]
struct Run {
    first: Option<i64>,
    last: Option<i64>,
    to: BasicBlock,
}

/// A jump table for a dense switch: the runs below `runs[lo]` and above `runs[hi]` are tested for (at most one on
/// each side), and the values of those from `lo` to `hi` go through the table.
#[derive(Clone, Copy, Debug)]
struct Table {
    lo: usize,
    hi: usize,
    cells: i64,
    instructions: i64,
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
                let mut sorted: Vec<(i64, BasicBlock)> = arms.iter().map(|(v, t)| (clamp(*v), *t)).collect();
                sorted.sort();
                if self.dense_switch(&vv, &sorted, *otherwise)? {
                    self.unsigned_switch = false;
                    return Ok(());
                }
                // without `otherwise` (`ranges::switch_domain`), the last arm is it
                let mut arms = arms.clone();
                let otherwise = match otherwise {
                    Some(o) => Some(*o),
                    None => arms.pop().map(|(_, b)| b),
                };
                for (value, target) in &arms {
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
                let l = self.label(other);
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

    /// A switch whose arms are on consecutive values (or, without `otherwise`, on every value it can be, as
    /// `ranges::switch_domain` leaves it), as runs of values that go to one block: a chain of tests, one SUB and one
    /// TAC a run, or (with the passes, at opt-level 1 and 2) a jump into a table, `ADD` of a `JMP` on the table's
    /// first cell, `STO` into the next cell and run it, into a row of `JMP`s in the code, one for each value. The
    /// table is taken when it is fewer cells, or as many and fewer instructions over the arms: 5 instructions for any
    /// value against two for each run passed, but a cell for each value, and a test for each side the values can be
    /// past. False when the arms aren't consecutive, and nothing is emitted.
    fn dense_switch(
        &mut self,
        vv: &Value,
        sorted: &[(i64, BasicBlock)],
        otherwise: Option<BasicBlock>,
    ) -> Result<bool, Fail> {
        let consecutive = sorted.windows(2).all(|w| w[1].0 == w[0].0 + 1);
        if sorted.len() < 2 || !consecutive {
            return Ok(false);
        }
        let (v0, vl) = (sorted[0].0, sorted[sorted.len() - 1].0);
        if !(-999..=999).contains(&v0) || !(-999..=999).contains(&vl) {
            return Ok(false);
        }
        // at opt-level 0 as it always was: a run each value, and a test for below the arms
        let opt = self.optimize;
        let mut runs: Vec<Run> = vec![];
        let mut push = |r: Run| match runs.last_mut() {
            Some(p) if opt && p.to == r.to => p.last = r.last,
            _ => runs.push(r),
        };
        if let Some(o) = otherwise {
            if !(opt && self.unsigned_switch && v0 <= 0) {
                push(Run {
                    first: None,
                    last: Some(v0 - 1),
                    to: o,
                });
            }
        }
        for &(v, to) in sorted {
            push(Run {
                first: Some(v),
                last: Some(v),
                to,
            });
        }
        if let Some(o) = otherwise {
            push(Run {
                first: Some(vl + 1),
                last: None,
                to: o,
            });
        }
        let comment = format!("switchInt == {v0} to {vl}");
        if runs.len() == 1 {
            self.carried = None;
            let l = self.label(runs[0].to);
            self.emit_c(Op::Jmp, l, &comment);
            return Ok(true);
        }
        // a new constant's cell, for the cost
        let kc = |l: &Self, k: i64| -> i64 { (k != 1 && k != 0 && !l.sh.consts.contains(&Sym::Const(k))) as i64 };
        // the chain: each run but the last a test, x - (its last + 1) < 0, from what the test before left
        let m = runs.len();
        let mut tests: Vec<i64> = vec![];
        let mut at = 0;
        for r in &runs[..m - 1] {
            let bound = r.last.unwrap() + 1;
            tests.push(bound - at);
            at = bound;
        }
        if tests.iter().any(|k| k.abs() > 999) {
            return Ok(false);
        }
        // (its last JMP left out: the block layout nearly always puts the last run's block next, and drops it)
        let chain_cells: i64 = tests.iter().map(|&k| if k != 0 { 2 + kc(self, k) } else { 1 }).sum();
        let run_of = |v: i64| {
            runs.iter()
                .position(|r| r.first.is_none_or(|f| f <= v) && r.last.is_none_or(|l| v <= l))
                .unwrap()
        };
        let chain_to = |i: usize| -> i64 {
            let passed: i64 = tests[..i.min(m - 1) + usize::from(i < m - 1)]
                .iter()
                .map(|&k| if k != 0 { 2 } else { 1 })
                .sum();
            passed + i64::from(i == m - 1)
        };
        let chain_instructions: i64 = (v0..=vl).map(|v| chain_to(run_of(v))).sum();
        let mut best: Option<Table> = None;
        if opt {
            for (a, b) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                if (runs[0].first.is_none() && a == 0) || (runs[m - 1].last.is_none() && b == 0) || m < a + b + 1 {
                    continue;
                }
                let (lo, hi) = (a, m - 1 - b);
                let (s, e) = (runs[lo].first.unwrap(), runs[hi].last.unwrap());
                let n = e - s + 1;
                // (the table's constant, a JMP on its cell `off` cells on, must be one the table's cells reach)
                let (mut cells, mut at, mut low, mut high) = (n + 4, 0, 0, 0);
                if a == 1 {
                    let k = s - at;
                    low = 1 + i64::from(k != 0);
                    cells += 1 + if k != 0 { 1 + kc(self, k) } else { 0 };
                    at = s;
                }
                if b == 1 {
                    let k = e + 1 - at;
                    high = 1 + i64::from(k != 0);
                    cells += 2 + if k != 0 { 1 + kc(self, k) } else { 0 };
                    at = e + 1;
                }
                let mut index = 4;
                if at < s {
                    cells += 1 + kc(self, s - at);
                    index += 1;
                    at = s;
                }
                if at - s > 99 || n > 99 {
                    continue;
                }
                let instructions: i64 = (v0..=vl)
                    .map(|v| {
                        let i = run_of(v);
                        if i < lo {
                            low
                        } else if i > hi {
                            low + high + 1
                        } else {
                            low + high + index
                        }
                    })
                    .sum();
                let t = Table {
                    lo,
                    hi,
                    cells,
                    instructions,
                };
                if best.is_none_or(|x| (t.cells, t.instructions) < (x.cells, x.instructions)) {
                    best = Some(t);
                }
            }
        }
        let table = best.filter(|t| (t.cells, t.instructions) < (chain_cells, chain_instructions));
        self.load(vv)?;
        let mut comment = comment;
        let Some(t) = table else {
            for (r, &k) in runs.iter().zip(&tests) {
                if k != 0 {
                    let c = self.konst(k);
                    self.emit_c(Op::Sub, c, &std::mem::take(&mut comment));
                }
                let l = self.label(r.to);
                self.emit_c(Op::Tac, l, &std::mem::take(&mut comment));
            }
            let l = self.label(runs[m - 1].to);
            self.emit(Op::Jmp, l);
            return Ok(true);
        };
        comment.push_str(", through a jump table");
        let (s, e) = (runs[t.lo].first.unwrap(), runs[t.hi].last.unwrap());
        let mut at = 0;
        if t.lo == 1 {
            if s != at {
                let c = self.konst(s - at);
                self.emit_c(Op::Sub, c, &std::mem::take(&mut comment));
            }
            let l = self.label(runs[0].to);
            self.emit_c(Op::Tac, l, &std::mem::take(&mut comment));
            at = s;
        }
        if t.hi == m - 2 {
            if e + 1 != at {
                let c = self.konst(e + 1 - at);
                self.emit_c(Op::Sub, c, &std::mem::take(&mut comment));
            }
            let go = self.fresh("jg");
            self.emit_c(Op::Tac, go.clone(), &std::mem::take(&mut comment));
            let l = self.label(runs[m - 1].to);
            self.emit(Op::Jmp, l);
            self.define(&go);
            at = e + 1;
        }
        if at < s {
            let c = self.konst(s - at);
            self.emit_c(Op::Sub, c, &std::mem::take(&mut comment));
            at = s;
        }
        // the table in the code, after the cell the jump into it is built in, and the JMP on its first cell after
        // it: the data is assembled on its own, and can't name a label in the code
        let row = self.fresh("jt");
        let jump = Sym::Instr(Access::Jump, Box::new(Cell::element(&row, (at - s) as u32)));
        self.emit_c(Op::Add, jump.clone(), &std::mem::take(&mut comment));
        let slot = Sym::Slot(Box::new(row.clone()));
        self.emit(Op::Sto, slot.clone());
        let mut go = Line::data_cell(&slot.to_string(), Arg::Num("0".into()));
        go.note("becomes JMP into the table");
        self.code.push(go);
        for v in s..=e {
            let entry = self.table_entry(runs[run_of(v)].to);
            let line = if v == s {
                let mut first = Line::data_cell(&row.to_string(), entry);
                first.note(&format!("the table: {s} to {e}"));
                first
            } else {
                Line::new(Kind::Dat(None, entry))
            };
            self.code.push(line);
        }
        self.code
            .push(Line::data_cell(&jump.to_string(), jump.contents().unwrap()));
        Ok(true)
    }

    /// A jump table's entry for a block: `JMP` to it, or the instruction the block is when it does nothing else, so
    /// that the table goes there itself: past blocks that only go to another, to the block they go to, and a return
    /// as the return (`HRS 00` in main).
    fn table_entry(&self, to: BasicBlock) -> Arg {
        let mut b = to;
        let mut seen = HashSet::new();
        while let Some(blk) = self.blocks.iter().find(|x| x.name == b) {
            if b == BasicBlock(0) || !blk.statements.iter().all(|s| *s == Statement::Nop) || !seen.insert(b) {
                break;
            }
            match blk.terminator {
                Terminator::Goto(n) => b = n,
                Terminator::Return if self.func == "main" => return Arg::Num("900".into()),
                Terminator::Return => return Arg::Sym(Sym::Exit(self.func.clone()).to_string(), Some(800)),
                _ => break,
            }
        }
        Arg::Sym(self.label(b).to_string(), Some(800))
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
