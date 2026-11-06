//! Places and values: where an operand is, how it comes into the accumulator and goes back out, and the
//! self-modifying cells that reach an address worked out at run time.
//!
//! A place is either a named cell or an address computed at run time, which the code leaves in the accumulator.
//! Reading or writing a computed place is self-modifying code: add 100 (CLA) or 600 (STO) to the address, store that
//! instruction in the next cell, and let it run.
use crate::compile::ir::{self, BinOp, Const, IntTy, Local, Projection, Rvalue, Segment, Statement, UnOp};
use crate::compile::machine::Op;
use crate::compile::Fail;

use super::{bare, is_comparison, numbered, one_word, Access, Cell, Lowering, Operand, Sym};

/// A place whose address is worked out at run time: the code leaves the address (or, given an access, the whole
/// instruction) in the accumulator.
#[derive(Clone)]
pub(super) enum Dyn {
    /// `*r`
    Deref(Sym),
    /// `(*r)[k]`
    DerefAt(Sym, i64),
    /// `xs[i + k]`
    IndexPlus(Sym, Sym, u32),
    /// `xs[i]`
    Index(Sym, Sym),
    /// `(*r)[i]`
    DerefIndex(Sym, Sym),
}

/// Where a value goes or comes from: a named cell, or a computed address.
#[derive(Clone)]
pub(super) enum Loc {
    At(Cell),
    Computed(Dyn),
}

/// An operand as lowering sees it: a constant, a named cell, or a computed address.
#[derive(Clone)]
pub(super) enum Value {
    Const(i64),
    At(Cell),
    Computed(Dyn),
}

impl Value {
    pub(super) fn k(&self) -> Option<i64> {
        if let Value::Const(k) = self {
            Some(*k)
        } else {
            None
        }
    }
}

/// The range of values a cell of this type holds.
pub(super) fn range_of(ty: IntTy) -> Option<(i64, i64)> {
    match ty {
        IntTy::U8 => Some((0, 255)),
        IntTy::I8 => Some((-128, 127)),
        IntTy::Usize => Some((0, 999)),
        IntTy::I16 => Some((-999, 999)),
        _ => None,
    }
}

/// `NAME`: the name of a `const` item.
fn is_const_name(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && cs.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// One plain segment that names a `const` item.
fn is_const_path(segments: &[Segment]) -> bool {
    matches!(segments, [Segment::Name { name, args: None }] if is_const_name(name))
}

/// The operand is `const k` for an integer k of one of these magnitudes (of any type).
fn int_const_of(o: &ir::Operand, magnitudes: &[i128], negative_too: bool) -> bool {
    matches!(o.int(), Some((v, _)) if magnitudes.contains(&v.abs()) && (negative_too || v >= 0))
}

impl<'a> Lowering<'a> {
    pub(super) fn place_any(&mut self, p: &ir::Place) -> Result<Loc, Fail> {
        if let ir::PlaceBase::Result { func, local } = &p.base {
            // a callee's cell (@f.2 is f's _2): its struct result
            if p.projection.is_empty() {
                return Ok(Loc::At(Sym::local(func, Local(*local)).into()));
            }
            fail!("unsupported place {p}")
        }
        let Some(l) = p.base_local().and_then(numbered) else {
            fail!("unsupported place {p}")
        };
        match p.projection.as_slice() {
            [Projection::Field(k, t)] if *k < 10 && one_word(t) => Ok(Loc::At(self.cell(l, Some(*k)).into())),
            [] => Ok(Loc::At(self.cell(l, None).into())),
            [Projection::Deref] => Ok(Loc::Computed(Dyn::Deref(self.cell(l, None)))),
            // constant index: a fixed cell
            [Projection::ConstantIndex { offset, .. }] => {
                let base = self.cell(l, None);
                Ok(Loc::At(Cell::element(&base, *offset as u32)))
            }
            [Projection::Deref, Projection::ConstantIndex { offset, .. }] => {
                Ok(Loc::Computed(Dyn::DerefAt(self.cell(l, None), *offset as i64)))
            }
            // xs[i + k], k folded into the address
            [Projection::IndexPlus(i, k)] if numbered(*i).is_some() => {
                let (base, i) = (self.cell(l, None), self.cell(*i, None));
                if !self.arrays.has(&base) {
                    fail!("indexing a non-array {p}")
                }
                Ok(Loc::Computed(Dyn::IndexPlus(base, i, *k)))
            }
            [Projection::Index(i)] if numbered(*i).is_some() => {
                let (base, i) = (self.cell(l, None), self.cell(*i, None));
                if !self.arrays.has(&base) {
                    fail!("indexing a non-array {p}")
                }
                Ok(Loc::Computed(Dyn::Index(base, i)))
            }
            [Projection::Deref, Projection::Index(i)] if numbered(*i).is_some() => {
                let (r, i) = (self.cell(l, None), self.cell(*i, None));
                Ok(Loc::Computed(Dyn::DerefIndex(r, i)))
            }
            _ => fail!("unsupported place {p}"),
        }
    }

    /// The code for a computed address: the address, or given an access the whole instruction for an array
    /// element. Returns whether it made the whole instruction.
    pub(super) fn run_dyn(&mut self, d: &Dyn, access: Option<Access>) -> bool {
        match d {
            Dyn::Deref(r) => {
                self.emit(Op::Cla, r.clone());
                false
            }
            Dyn::DerefAt(r, k) => {
                self.emit(Op::Cla, r.clone());
                if *k != 0 {
                    let c = self.konst(*k);
                    self.emit(Op::Add, c);
                }
                false
            }
            Dyn::IndexPlus(base, i, k) => {
                self.emit(Op::Cla, i.clone());
                let at = Cell {
                    sym: base.clone(),
                    offset: Some(*k),
                };
                let c = match access {
                    Some(access) => self.addr_op(at, access),
                    None => self.addr(at),
                };
                self.emit(Op::Add, c);
                access.is_some()
            }
            Dyn::Index(base, i) => {
                // the instruction itself in one constant: CLA base (100 + base) or STO base (600 + base), plus the
                // index
                self.emit(Op::Cla, i.clone());
                let at = Cell::from(base.clone());
                let c = match access {
                    Some(access) => self.addr_op(at, access),
                    None => self.addr(at),
                };
                self.emit(Op::Add, c);
                access.is_some()
            }
            Dyn::DerefIndex(r, i) => {
                self.emit(Op::Cla, i.clone());
                self.emit(Op::Add, r.clone());
                false
            }
        }
    }

    pub(super) fn place(&mut self, p: &ir::Place) -> Result<Cell, Fail> {
        match self.place_any(p)? {
            Loc::At(c) => Ok(c),
            Loc::Computed(_) => fail!("expected a plain local, got {p}"),
        }
    }

    /// A constant cell holding an instruction on a cell: CLA, STO or OUT.
    fn addr_op(&mut self, on: Cell, access: Access) -> Sym {
        let c = Sym::Instr(access, Box::new(on));
        self.sh.constant(&c);
        c
    }

    /// A constant cell holding a cell's address.
    pub(super) fn addr(&mut self, of: Cell) -> Sym {
        let c = Sym::Address(Box::new(of));
        self.sh.constant(&c);
        c
    }

    /// The accumulator holds an address (or, folded, the instruction): build the instruction in a cell of its own,
    /// and return the cell, which the caller places where it is to run.
    pub(super) fn selfmod(&mut self, access: Access, comment: &str, folded: bool) -> Sym {
        let cell = self.fresh("sm");
        if !folded {
            let c = self.konst(access.opcode());
            self.emit_c(Op::Add, c, comment);
        }
        self.emit_c(Op::Sto, cell.clone(), if folded { comment } else { "" });
        cell
    }

    pub(super) fn operand(&mut self, o: &ir::Operand) -> Result<Value, Fail> {
        if let ir::Operand::Constant(c) = o {
            match c {
                Const::Int {
                    value,
                    ty: ty @ (IntTy::U8 | IntTy::I8 | IntTy::Usize | IntTy::I16),
                } => {
                    let v = *value as i64;
                    let (lo, hi) = range_of(*ty).unwrap();
                    // overflow known at compile time is an error, as rustc's `arithmetic_overflow` makes it for an
                    // i16: here past a card, which rustc can't see
                    if v < lo || v > hi {
                        fail!(
                            "constant out of range: {v} is past what a card holds ({lo} to {hi}): a literal that size, \
                             or arithmetic rustc worked out to it at compile time, would overflow a card"
                        )
                    }
                    return Ok(Value::Const(v));
                }
                // a static's address: the cell holding it, a constant (see `front::Statics`)
                Const::Alloc(alloc, _) => {
                    let Some(name) = self.sh.statics.of_alloc.get(alloc).cloned() else {
                        fail!("unsupported place {c}: a pointer into memory this backend doesn't have (a static's is, when its first value is a literal of numbers or bools)")
                    };
                    if !self.sh.statics_used.contains(&name) {
                        self.sh.statics_used.push(name.clone());
                    }
                    let a = self.addr(Cell::from(Sym::Static(name)));
                    return Ok(Value::At(Cell::from(a)));
                }
                // i16::MIN and MAX only turn up in rustc's own overflow checks; no card can equal them
                Const::Path(p) if p.is_path(&["i16", "MIN"]) => return Ok(Value::Const(-32768)),
                Const::Path(p) if p.is_path(&["i16", "MAX"]) => return Ok(Value::Const(32767)),
                // a named `const` item, resolved from the MIR
                Const::Path(p) if p.qself.is_none() && is_const_path(&p.segments) => {
                    let name = p.name();
                    return match self.sh.named.get(name) {
                        Some(v) => Ok(Value::Const(*v)),
                        None => fail!("unknown const {name}"),
                    };
                }
                Const::Bool(true) => return Ok(Value::Const(1)),
                Const::Bool(false) => return Ok(Value::Const(0)),
                _ => fail!("unsupported place {}", bare(o)),
            }
        }
        let Some(p) = o.place() else {
            fail!("unsupported place {o}")
        };
        Ok(match self.place_any(p)? {
            Loc::Computed(d) => Value::Computed(d),
            Loc::At(c) => Value::At(c),
        })
    }

    // Accumulator forwarding: a temporary made by one statement and read once, as the first operand of the very
    // next, stays in the accumulator: not stored, not loaded back. Then a value a card cannot hold (5 p before / 10,
    // say) is still exact there, as it is in native Rust; and two cells are saved.
    pub(super) fn carries(&mut self, s: &Statement, next: Option<&Statement>, into_branch: bool) -> Option<Cell> {
        let Statement::Assign(d, rv) = s else { return None };
        let t = d.as_local().and_then(numbered)?;
        let second: Option<&ir::Operand> = match rv {
            Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem, _, b) => Some(b),
            Rvalue::UnaryOp(UnOp::Neg, _) | Rvalue::Lin(_) => None,
            _ => return None,
        };
        let next = next?;
        if !self.pooled.contains(&t) {
            return None;
        }
        // a usize that can be past a card is tested for its sign, from its cell, before it is used
        if self.huge.contains(&t) || ir::Locals::locals(next).iter().any(|l| self.huge.contains(l)) {
            return None;
        }
        // only shifts: no routine in between (a Rem by 10 or 100 is one shift too, but it is not a producer here: it
        // keeps the value small anyway)
        if matches!(rv, Rvalue::BinaryOp(BinOp::Div | BinOp::Rem, ..))
            && !second.is_some_and(|b| int_const_of(b, &[10, 100], true))
        {
            return None;
        }
        if ir::occurrences(&self.blocks, t) != 2 {
            return None;
        }
        let Statement::Assign(nd, nrv) = next else { return None };
        // into a comparison with a constant (it loads the other side first: see branch_on)
        if let Rvalue::BinaryOp(op, a, b) = nrv {
            if is_comparison(*op) && nd.as_local().and_then(numbered).is_some() {
                if !into_branch {
                    return None; // only a comparison fused into its branch loads that side first
                }
                let is_t =
                    |v: &ir::Operand| matches!(v, ir::Operand::Copy(_) | ir::Operand::Move(_)) && v.local() == Some(t);
                let konst = |v: &ir::Operand| v.int().is_some();
                if (is_t(a) && konst(b)) || (is_t(b) && konst(a)) {
                    return Some(self.cell(t, None).into());
                }
                return None;
            }
        }
        let Rvalue::BinaryOp(
            op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem),
            ir::Operand::Move(first),
            u4,
        ) = nrv
        else {
            return None;
        };
        nd.as_local().and_then(numbered)?;
        if first.as_local() != Some(t) {
            return None;
        }
        if *op == BinOp::Rem && !int_const_of(u4, &[10, 100], false) {
            return None;
        }
        if *op == BinOp::Mul && !int_const_of(u4, &[1, 10, 100], true) {
            return None; // by any other constant it re-reads the operand
        }
        if *op == BinOp::Div && !int_const_of(u4, &[10, 100], true) {
            return None;
        }
        // a second operand read through a computed address is read before the first is loaded, and that reading
        // takes the accumulator: t must be in its cell by then
        if u4.place().is_some_and(|p| {
            p.projection
                .iter()
                .any(|pr| matches!(pr, Projection::Deref | Projection::Index(_) | Projection::IndexPlus(..)))
        }) {
            return None;
        }
        // the second operand is not t: t is named twice only, where it is made and here as the first operand
        Some(self.cell(t, None).into())
    }

    /// The value into the accumulator.
    pub(super) fn load(&mut self, op: &Value) -> Result<(), Fail> {
        match op {
            Value::At(c) if self.carried.as_ref() == Some(c) => {
                self.carried = None; // it is already in the accumulator
            }
            Value::Computed(d) => {
                // read through a computed address
                let f = self.run_dyn(d, Some(Access::Read));
                let c = self.selfmod(Access::Read, "read through address", f);
                self.cell_here(&c); // becomes CLA <address>
            }
            Value::Const(k) => {
                let c = self.konst(*k);
                self.emit(Op::Cla, c);
            }
            Value::At(c) => self.emit(Op::Cla, c.clone()),
        }
        Ok(())
    }

    /// A value read through a computed address, into a scratch cell first: an operand of ADD or SUB must be a cell.
    pub(super) fn mat(&mut self, op: Value) -> Value {
        if !matches!(op, Value::Computed(_)) {
            return op;
        }
        let t = self.temp();
        let _ = self.load(&op);
        self.emit(Op::Sto, t.clone());
        Value::At(t.into())
    }

    /// The cell holding a materialized value.
    pub(super) fn arith(&mut self, op: &Value) -> Result<Operand, Fail> {
        match op {
            Value::Computed(_) => fail!("internal: unmaterialized operand"),
            Value::Const(k) => Ok(self.konst(*k)),
            Value::At(c) => Ok(c.clone().into()),
        }
    }

    /// The value is in the accumulator.
    pub(super) fn store(&mut self, dst: &Loc, comment: &str) -> Result<(), Fail> {
        match dst {
            Loc::At(c) if self.carry.as_ref() == Some(c) => {
                self.carried = Some(c.clone()); // left there for the next statement
            }
            Loc::At(c) => self.emit_c(Op::Sto, c.clone(), comment),
            Loc::Computed(d) => {
                let v = self.temp();
                self.emit_c(Op::Sto, v.clone(), comment);
                let f = self.run_dyn(d, Some(Access::Write));
                let c = self.selfmod(Access::Write, "write through address", f);
                self.emit(Op::Cla, v);
                self.cell_here(&c); // becomes STO <address>
            }
        }
        Ok(())
    }

    /// An argument into the cell it is passed in, unless it is there already (a parameter passed straight on).
    pub(super) fn pass(&mut self, op: &Value, to: &Sym, comment: &str) -> Result<(), Fail> {
        if matches!(op, Value::At(c) if c.offset.is_none() && c.sym == *to) {
            return Ok(());
        }
        self.load(op)?;
        self.emit_c(Op::Sto, to.clone(), comment);
        Ok(())
    }
}
