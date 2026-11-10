//! Lowering (instruction selection): one function's blocks, after the passes, to CARDIAC instructions.
//!
//! Calls use CARDIAC's own convention. JMP leaves a return instruction (8xx) in cell 99; each function's prologue
//! copies cell 99 into its exit cell, and `return` jumps to that cell. Every function has a fixed set of cells (its
//! locals are cells named after it), so there is no recursion.
//!
//! Here: the function's layout, its cells and its data section. `symbols` names the cells; `places` reads and writes
//! them (and the addresses worked out at run time); `statements` lowers each statement, `arith` the arithmetic and
//! its checks, `branches` the switches, asserts and compare-and-branch, and `calls` the calls and the built-ins.
use std::collections::{HashMap, HashSet};

use super::ir::{self, lines_naming, BasicBlock, BinOp, IntTy, Local, Segment, Statement, Terminator, Ty, Unwind};
use super::machine::{Arg, Kind, Line, Op};
use super::util::Ordered;
use super::Fail;

/// `fail!(name(...))`: return the error a constructor of `codes` makes.
macro_rules! fail { ($($t:tt)*) => { return Err(crate::codes::$($t)*) } }

mod arith;
mod branches;
pub(crate) mod calls;
mod places;
mod statements;
mod symbols;

pub use self::symbols::{Access, Cell, Operand, Sym};

pub const PANIC: i64 = -999;

/// The scratch pool. A temporary defined and used inside one basic block never outlives it, and a call only
/// happens at a block's end, after its arguments are read. With no recursion, every block of every function can
/// share one pool of cells, reset at the start of each block. It also notes which shared routines the program
/// needs.
#[derive(Default)]
pub struct Pool {
    pub size: usize,
    next: usize,
    pub panic: bool,
    pub mul: bool,
    pub mul_checked: bool,
    pub div: bool,
    pub divr: bool,
    pub chk: bool,
    /// skip_cards(): the row of INP 00
    pub skip: bool,
    /// add_times(): the row of ADD c
    pub times: bool,
    /// `&`, `|` and `^` between integers, each a routine of its own
    pub bitwise: Vec<super::runtime::BitRoutine>,
    /// `>>` and `<<` by a variable: an i16's or a u8's `>>`, an i16's `<<`, a u8's `<<`
    pub shr: bool,
    pub shl: bool,
    pub shl8: bool,
}

impl Pool {
    fn reset(&mut self) {
        self.next = 0;
    }

    fn take(&mut self) -> Sym {
        let n = self.next;
        self.next += 1;
        self.size = self.size.max(self.next);
        Sym::Scratch(n)
    }
}

/// What every function's lowering adds to: the pool, the constant cells (in the order they were first used), where
/// each phase starts, and the named scalar consts.
pub struct Shared {
    pub pool: Pool,
    pub consts: Vec<Sym>,
    pub phase_starts: Vec<String>,
    pub named: HashMap<String, i64>,
    /// the resources the program loads, each once
    pub resources: Vec<String>,
    /// each test of a bool kept in a cell (`CLA b / SUB 00 / TAC`), by the variable's name where it has one: what
    /// `cardiac::Flag` would make an instruction less
    pub bool_tests: Vec<Option<String>>,
    /// the program's statics, and those a function points into, in the order they are first used
    pub statics: super::front::Statics,
    pub statics_used: Vec<String>,
    /// the grids laid out by tens (`nested::arrays_of_arrays`), whose holes nothing writes
    pub grids: super::nested::Holes,
    /// each such grid's first cell, as the listing labels it, and its holes, counted from there: free cells that
    /// `holes::fill` gives to the program's other cells
    pub holes: Vec<(String, Vec<usize>)>,
}

impl Shared {
    /// A constant cell the program needs.
    fn constant(&mut self, c: &Sym) {
        if !self.consts.contains(c) {
            self.consts.push(c.clone());
        }
    }
}

/// A function's parameters, in order, and its return type.
#[derive(Clone)]
pub struct Sig {
    pub params: Vec<Local>,
    /// how many cells each parameter takes: an array passed by value its length, anything else one
    pub widths: Vec<u64>,
    pub ret: Ty,
}

/// What a call needs to know of the program's functions: each one's signature, and the parameters passed straight
/// on, each in the cell it shares (copies::forwarded).
pub struct Callees {
    sigs: HashMap<String, Sig>,
    forwarded: HashMap<Sym, Sym>,
}

impl Callees {
    /// The signatures, and the forwarded parameters, each with the cell it shares (copies::forwarded).
    pub fn new<'s>(sigs: impl IntoIterator<Item = (&'s String, &'s Sig)>, forwarded: &HashMap<Sym, Sym>) -> Self {
        let sigs: HashMap<String, Sig> = sigs
            .into_iter()
            .map(|(name, sig)| (name.clone(), sig.clone()))
            .collect();
        Callees {
            sigs,
            forwarded: forwarded.clone(),
        }
    }

    /// The cell a local lives in: its own, or the one it is passed on in.
    fn home(&self, cell: Sym) -> Sym {
        self.forwarded.get(&cell).cloned().unwrap_or(cell)
    }
}

/// An array local: consecutive cells. A grid laid out by tens has holes between its rows, which nothing writes.
struct Array {
    len: usize,
    elem: Ty,
    /// a grid by tens: its width and stride
    gaps: Option<(usize, usize)>,
}

impl Array {
    /// Is element k a hole between two of a grid's rows?
    fn hole(&self, k: usize) -> bool {
        self.gaps.is_some_and(|(w, s)| k % s >= w)
    }
}

/// A type printed as one word: `i16`, `bool`, `Point`.
fn one_word(t: &Ty) -> bool {
    match t {
        Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str => true,
        Ty::Path(p) => p.qself.is_none() && matches!(p.segments.as_slice(), [Segment::Name { args: None, .. }]),
        _ => false,
    }
}

/// The local, unless it is the struct pass's `_unit`.
fn numbered(l: Local) -> Option<Local> {
    (l != Local::UNIT).then_some(l)
}

/// What an operand reads, without saying how: `copy _5` is `_5`.
fn bare(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Copy(p) | ir::Operand::Move(p) | ir::Operand::Place(p) => p.to_string(),
        ir::Operand::Constant(c) => format!("const {c}"),
        ir::Operand::Aggregate(..) | ir::Operand::Fn(_) => o.to_string(),
    }
}

/// A statement as a comment: its text without the semicolon.
fn source(s: &Statement) -> String {
    s.to_string().trim_end_matches(';').to_string()
}

/// A plain unwind: `unwind continue` or `unwind unreachable`.
fn plain_unwind(u: &Unwind) -> bool {
    matches!(u, Unwind::Continue | Unwind::Unreachable)
}

/// A name of letters, digits and underscores.
/// What Rust says when a check fails, from the message rustc's MIR gives the check: `attempt to compute `{} + {}`,
/// which would overflow` is `attempt to add with overflow`, as a debug build panics. The values are left out (the
/// map is written before the program runs); a message this doesn't know is given as the MIR has it.
pub fn check_message(msg: &str) -> String {
    let overflow = |operator: &str| {
        let operation = match operator {
            "+" => "add",
            "-" => "subtract",
            "*" => "multiply",
            "/" => "divide",
            _ => return None,
        };
        Some(format!("attempt to {operation} with overflow"))
    };
    if msg.starts_with("index out of bounds") {
        return "index out of bounds".into();
    }
    if let Some(rest) = msg.strip_prefix("attempt to compute `{} ") {
        if let Some(message) = rest.split(' ').next().and_then(overflow) {
            return message;
        }
    }
    match msg {
        "attempt to compute the remainder of `{} % {}`, which would overflow" => {
            "attempt to calculate the remainder with overflow".into()
        }
        "attempt to negate `{}`, which would overflow" => "attempt to negate with overflow".into(),
        "attempt to shift left by `{}`, which would overflow" => "attempt to shift left with overflow".into(),
        "attempt to shift right by `{}`, which would overflow" => "attempt to shift right with overflow".into(),
        "attempt to divide `{}` by zero" => "attempt to divide by zero".into(),
        "attempt to calculate the remainder of `{}` with a divisor of zero" => {
            "attempt to calculate the remainder with a divisor of zero".into()
        }
        _ => msg.into(),
    }
}

/// The blocks on a loop: those a path from one of its successors leads back to.
fn on_a_loop(blocks: &[ir::Block]) -> HashSet<BasicBlock> {
    let successors: HashMap<BasicBlock, Vec<BasicBlock>> = blocks
        .iter()
        .map(|block| (block.name, block.terminator.successors()))
        .collect();
    let mut looping = HashSet::new();
    for block in blocks {
        let mut to_visit = successors[&block.name].clone();
        let mut visited = HashSet::new();
        while let Some(reached) = to_visit.pop() {
            if reached == block.name {
                looping.insert(block.name);
                break;
            }
            if visited.insert(reached) {
                to_visit.extend(successors.get(&reached).into_iter().flatten().copied());
            }
        }
    }
    looping
}

fn is_word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

pub struct Lowering<'a> {
    optimize: bool,
    sh: &'a mut Shared,
    callees: &'a Callees,
    func: String,
    locals: Ordered<Local, Ty>,
    /// the locals that live in the scratch pool
    pooled: HashSet<Local>,
    /// the pool cell each pooled local (or field of one) has in the block being lowered
    slots: HashMap<(Local, Option<u32>), Sym>,
    code: Vec<Line>,
    /// numbers the labels and cells the function makes up
    tmp: usize,
    /// the function's one-cell locals and fields
    cells: HashSet<Sym>,
    arrays: Ordered<Sym, Array>,
    /// each overflow pair's integer type
    width: HashMap<Local, IntTy>,
    /// the cells that start other than 0
    init: HashMap<Cell, i64>,
    blocks: Vec<ir::Block>,
    /// the overflow flags whose op was checked when it ran
    fused_flags: HashSet<Local>,
    init_block: bool,
    /// the arrays set from constants by one statement and written by nothing else: their initial value, wherever
    /// the statement is
    constant_arrays: HashSet<Local>,
    /// whether the block being lowered is on a loop, so may run more than once
    in_loop: bool,
    /// the check a branch to rt_panic is taken for, in the block being lowered (see `Line::panic`)
    panic: Option<String>,
    seen: HashSet<Local>,
    /// the locals an overflow-checked op stored into straight from the block before, where their own block's move
    /// out of the pair became a nop (see `move_out`): written already, so not a cell's initial value to set
    moved_out: HashSet<Local>,
    /// the cell the statement being lowered leaves in the accumulator instead (see `carries`)
    carry: Option<Cell>,
    /// the cell whose value is in the accumulator now, not stored
    carried: Option<Cell>,
    /// the statement being lowered leaves a value in the accumulator that may be past a card: its store is marked
    /// (see `Line::wide`)
    wide: bool,
    /// the plain sums, differences and products whose result may be past a card (see `ranges::past_a_card`), and
    /// the block and index of the statement being lowered
    pub past: HashSet<(BasicBlock, usize)>,
    at: (BasicBlock, usize),
    /// the switch being lowered is on a bool or a u8, wherever it is read from: never negative (see `signed`)
    unsigned_switch: bool,
    /// the arrays the loader fills, by the resource each one is
    resources: HashMap<Sym, String>,
    /// the locals the source names, by name
    pub names: Ordered<String, Local>,
    /// the usize locals that can hold a value past a card, below 0 (see `ranges::huge_usizes`)
    pub huge: HashSet<Local>,
    /// the usizes a promise keeps on a card (`blocks::promised_cards`): a cast into one needs no test of its sign
    pub promised: HashSet<Local>,
    /// of those, the ones a passed check has shown are on a card in the block being lowered, until it sets them
    /// (see `checked_on_card`)
    on_card_now: HashSet<Local>,
    /// the function's result cells besides `_0`, which its caller reads after it returns
    pub results: HashSet<Local>,
}

impl<'a> Lowering<'a> {
    pub fn new(
        func: &str,
        locals: Ordered<Local, Ty>,
        sh: &'a mut Shared,
        callees: &'a Callees,
        optimize: bool,
        pooled: HashSet<Local>,
    ) -> Result<Self, Fail> {
        let mut l = Lowering {
            optimize,
            sh,
            callees,
            func: func.to_string(),
            locals: Ordered::new(),
            pooled,
            slots: HashMap::new(),
            code: vec![],
            tmp: 0,
            cells: HashSet::new(),
            arrays: Ordered::new(),
            width: HashMap::new(),
            init: HashMap::new(),
            blocks: vec![],
            fused_flags: HashSet::new(),
            init_block: false,
            constant_arrays: HashSet::new(),
            in_loop: false,
            panic: None,
            moved_out: HashSet::new(),
            seen: HashSet::new(),
            carry: None,
            carried: None,
            wide: false,
            past: HashSet::new(),
            at: (BasicBlock(0), 0),
            unsigned_switch: false,
            resources: HashMap::new(),
            names: Ordered::new(),
            huge: HashSet::new(),
            promised: HashSet::new(),
            on_card_now: HashSet::new(),
            results: HashSet::new(),
        };
        for (&local, ty) in locals.iter() {
            if *ty == Ty::Unit || *ty == Ty::Never {
                continue; // unit values need no cell, nor does `!`, which has none
            }
            // an overflow-checked pair's type says how its op is checked, wherever its cells are
            if let Some(width) = overflow_pair(ty) {
                l.width.insert(local, width);
            }
            if l.pooled.contains(&local) {
                continue; // lives in the shared scratch pool
            }
            if l.callees.forwarded.contains_key(&Sym::local(func, local)) {
                continue; // a parameter in the cell it is passed on in
            }
            // One cell holds a u8, an i16 (a card value, panicking past +-999), a bool, a usize (an index, panicking
            // above 999), or a reference or raw pointer, which is a cell address 00-99. An array's element is one of
            // them too, but a usize past a card: only a local holds one (`ranges::huge_usizes`), and storing one
            // anywhere else panics. A slice's pointer is its address too: its length is a local of its own
            // (`slices::run_time_lengths`); and so is a reference to a cell that holds a reference (`find`'s closure
            // takes `&&i16`).
            let card = |t: &Ty| matches!(t, Ty::Int(IntTy::U8 | IntTy::I8 | IntTy::I16));
            let element = |t: &Ty| matches!(t, Ty::Int(IntTy::U8 | IntTy::I8 | IntTy::I16 | IntTy::Usize) | Ty::Bool);
            let pointee = |t: &Ty| {
                card(t)
                    || matches!(t, Ty::Array(e, ir::ArrayLen::Known(_)) if element(e))
                    || matches!(t, Ty::Slice(e) if element(e))
                    || matches!(t, Ty::Ref { to, .. } | Ty::Ptr { to, .. } if card(to))
            };
            let one_cell = match ty {
                Ty::Int(IntTy::U8 | IntTy::I8 | IntTy::I16 | IntTy::Usize) | Ty::Bool => true,
                Ty::Ref { lifetime: None, to, .. } | Ty::Ptr { to, .. } => pointee(to),
                _ => false,
            };
            if one_cell {
                let c = l.cell(local, None);
                l.cells.insert(c);
                continue;
            }
            if overflow_pair(ty).is_some() {
                let (a, b) = (l.cell(local, Some(0)), l.cell(local, Some(1)));
                l.cells.insert(a);
                l.cells.insert(b);
                continue;
            }
            match ty {
                Ty::Array(e, ir::ArrayLen::Known(n)) if element(e) => {
                    // consecutive cells
                    let c = l.cell(local, None);
                    l.arrays.set(
                        c,
                        Array {
                            len: *n as usize,
                            elem: (**e).clone(),
                            gaps: l
                                .sh
                                .grids
                                .get(&(func.to_string(), local))
                                .map(|(w, s)| (*w as usize, *s as usize)),
                        },
                    );
                }
                _ => fail!(unsupported_type(ty, local)),
            }
        }
        l.locals = locals;
        Ok(l)
    }

    fn label(&self, bb: BasicBlock) -> Sym {
        Sym::Block(self.func.clone(), bb)
    }

    /// A local's cell, or one field's (main_1, add_0, add_3_1 ...): a shared scratch cell for a pooled local, and
    /// for a parameter passed straight on, the cell it is passed in.
    fn cell(&mut self, local: Local, field: Option<u32>) -> Sym {
        if self.pooled.contains(&local) {
            if let Some(slot) = self.slots.get(&(local, field)) {
                return slot.clone();
            }
            let slot = self.sh.pool.take();
            self.slots.insert((local, field), slot.clone());
            return slot;
        }
        self.callees.home(Sym::Local {
            func: self.func.clone(),
            local,
            field,
        })
    }

    /// A label or a cell of the function's own, not named before.
    fn fresh(&mut self, stem: &'static str) -> Sym {
        let s = Sym::Fresh {
            func: self.func.clone(),
            stem,
            n: self.tmp,
        };
        self.tmp += 1;
        s
    }

    /// The cell holding the constant v.
    fn konst(&mut self, v: i64) -> Operand {
        if v == 1 {
            return Operand::CELL_00;
        }
        let c = Sym::Const(v);
        self.sh.constant(&c);
        Operand::from(c)
    }

    fn emit(&mut self, op: Op, arg: impl Into<Arg>) {
        self.push(Line::instr(op, arg.into()));
    }

    /// An instruction with a comment, unless the comment is empty.
    fn emit_c(&mut self, op: Op, arg: impl Into<Arg>, comment: &str) {
        if comment.is_empty() {
            self.emit(op, arg)
        } else {
            self.push(Line::commented(op, arg.into(), comment))
        }
    }

    /// An instruction, a branch to rt_panic marked with the check it is taken for.
    fn push(&mut self, mut line: Line) {
        if line.to_panic() {
            line.panic = self.panic.clone();
        }
        self.code.push(line);
    }

    /// `name:`, here.
    fn define(&mut self, name: &Sym) {
        self.code.push(Line::label(&name.to_string()));
    }

    /// `name: DAT 0`, here: a cell in the code, which the code writes an instruction into before it runs it.
    fn cell_here(&mut self, name: &Sym) {
        self.code.push(Line::data_cell(&name.to_string(), Arg::Num("0".into())));
    }

    /// An error, with the MIR line it came from and the function the line is in, unless it already shows the line.
    fn at(&self, e: Fail, line: &str) -> Fail {
        if e.message.contains(line) {
            e
        } else {
            e.and(&format!(", in {}: `{line}`", self.func))
        }
    }

    fn count_lines(&self, l: Local) -> usize {
        lines_naming(&self.blocks, l)
    }

    /// The array locals one statement sets from constants and nothing else writes: not a parameter (its caller
    /// writes it), nothing else assigned to it or to an element, no call's result, and no address taken that could
    /// write through. Rust reads no local before it is set, so every read sees those constants, and they can be the
    /// cells' initial value on the tape, wherever the statement is and however often it runs.
    fn constant_arrays(&self) -> HashSet<Local> {
        let params = self
            .callees
            .sigs
            .get(&self.func)
            .map(|s| s.params.clone())
            .unwrap_or_default();
        let scalar = |o: &ir::Operand| matches!(o, ir::Operand::Constant(c) if c.is_scalar());
        let mut sets: HashMap<Local, usize> = HashMap::new();
        let mut written: HashSet<Local> = HashSet::new();
        for b in &self.blocks {
            for s in &b.statements {
                if let Statement::Assign(p, rv) = s {
                    let constant = match rv {
                        ir::Rvalue::Array(items) => items.iter().all(scalar),
                        ir::Rvalue::Repeat(o, _) => scalar(o),
                        _ => false,
                    };
                    match (p.as_local(), constant) {
                        (Some(x), true) => *sets.entry(x).or_default() += 1,
                        _ => written.extend(p.base_local()),
                    }
                    if let ir::Rvalue::Ref(k, q) = rv {
                        if *k != ir::BorrowKind::Shared {
                            written.extend(q.base_local());
                        }
                    }
                }
            }
            if let Terminator::Call { dest, .. } = &b.terminator {
                written.extend(dest.base_local());
            }
        }
        sets.into_iter()
            .filter(|(x, n)| {
                *n == 1
                    && !written.contains(x)
                    && !params.contains(x)
                    && !self.pooled.contains(x)
                    && matches!(self.locals.get(x), Some(Ty::Array(..)))
            })
            .map(|(x, _)| x)
            .collect()
    }

    /// Whether every read of `l` is of a value set earlier in the same block: none is carried from one block to
    /// another (rustc can work out one check in each of two branches, into the same local).
    fn never_carried(&self, l: Local) -> bool {
        self.blocks.iter().all(|b| {
            for line in b.each_line() {
                let (def, uses) = line.def_use();
                if uses.contains(&l) || (def != Some(l) && line.locals().contains(&l)) {
                    return false;
                }
                if def == Some(l) {
                    return true;
                }
            }
            true
        })
    }

    fn temp(&mut self) -> Sym {
        self.sh.pool.take()
    }

    /// One shared routine: punches -999, halts.
    fn use_panic(&mut self) -> Sym {
        self.konst(PANIC);
        self.sh.pool.panic = true;
        Sym::Runtime("rt_panic")
    }

    /// Is element k of the array at a cell a hole between a grid's rows, which nothing writes?
    fn hole_at(&self, c: &Cell, k: usize) -> bool {
        self.array_at(c).is_some_and(|a| a.hole(k))
    }

    /// The elements of the n-element array at a cell that hold something: all but a grid's holes.
    fn kept(&self, c: &Cell, n: u32) -> Vec<u32> {
        (0..n).filter(|k| !self.hole_at(c, *k as usize)).collect()
    }

    /// The array at a cell, if one starts there.
    fn array_at(&self, c: &Cell) -> Option<&Array> {
        if c.offset.is_some() {
            return None;
        }
        self.arrays.get(&c.sym)
    }

    /// Already lowered: too late to change.
    fn emitted(&self, b: BasicBlock) -> bool {
        let l = Kind::Label(self.label(b).to_string());
        self.code.iter().any(|x| x.kind == l)
    }

    /// Lower the function. Returns its code and its data lines.
    pub fn lower(&mut self, blocks: Vec<ir::Block>) -> Result<(Vec<Line>, Vec<Line>), Fail> {
        self.init.clear();
        self.moved_out.clear();
        self.blocks = blocks;
        self.fused_flags.clear();
        let entry = BasicBlock(0);
        let bb0_targeted = self.blocks.iter().any(|b| b.terminator.names(entry));
        // a function that returns `!` has no way back to keep
        let returns = self.func != "main" && self.callees.sigs.get(&self.func).is_none_or(|s| s.ret != Ty::Never);
        if self.func != "main" {
            self.define(&Sym::Entry(self.func.clone()));
        }
        if returns {
            let f = self.func.clone();
            self.emit_c(
                Op::Cla,
                Operand::CELL_99,
                &format!("fn {f}: keep the return instruction"),
            );
            self.emit(Op::Sto, Sym::Exit(f));
        }
        // main's opening run of blocks, each entered only from the one before (a call's return, a goto, a passed
        // assert): they run exactly once, before anything else, so a constant they store first can be the cell's
        // initial value on the tape instead. So does the run that opens each later phase, after `overlay()`, for a
        // local no earlier phase names: its cell is punched by a tape before it is read, and nothing has touched it
        // since
        let mut once: Vec<usize> = vec![];
        let mut seen_before: HashMap<usize, HashSet<Local>> = HashMap::new();
        if self.optimize && self.func == "main" && !bb0_targeted {
            let index = |blocks: &[ir::Block], n: BasicBlock| blocks.iter().rposition(|x| x.name == n);
            let is_overlay = |t: &Terminator| matches!(t, Terminator::Call { func, args, .. } if func.name() == "overlay" && args.is_empty());
            // where runs start: main's entry, and the block each `overlay()` returns to, if nothing else leads there
            let mut starts: Vec<usize> = index(&self.blocks, entry).into_iter().collect();
            for b in self.blocks.iter() {
                if let Terminator::Call { target: Some(n), .. } = &b.terminator {
                    if is_overlay(&b.terminator) && ir::blocks_naming(&self.blocks, *n) == 1 {
                        starts.extend(index(&self.blocks, *n));
                    }
                }
            }
            let mut first = true;
            for s in starts {
                let mut run: Vec<usize> = vec![];
                let mut b = Some(s);
                while let Some(bi) = b {
                    if once.contains(&bi) || run.contains(&bi) {
                        break;
                    }
                    run.push(bi);
                    let t = &self.blocks[bi].terminator;
                    let nx = match t {
                        Terminator::Goto(n) => Some(*n),
                        Terminator::Call {
                            target: Some(n),
                            unwind,
                            ..
                        } if plain_unwind(unwind) => Some(*n),
                        Terminator::Assert { target, unwind, .. } if plain_unwind(unwind) => Some(*target),
                        _ => None,
                    };
                    let Some(nx) = nx else { break };
                    if ir::blocks_naming(&self.blocks, nx) != 1 || is_overlay(t) {
                        break;
                    }
                    b = index(&self.blocks, nx);
                }
                // what runs before this run: nothing for main's own; for a phase's, every block reached from main's
                // entry without coming to the run
                let mut seen: HashSet<Local> = HashSet::new();
                if !first {
                    let mut reached: HashSet<usize> = HashSet::new();
                    let mut todo: Vec<usize> = index(&self.blocks, entry).into_iter().collect();
                    while let Some(i) = todo.pop() {
                        if i == run[0] || !reached.insert(i) {
                            continue;
                        }
                        for n in self.blocks[i].terminator.successors() {
                            todo.extend(index(&self.blocks, n));
                        }
                    }
                    for &i in &reached {
                        for l in self.blocks[i].lines() {
                            seen.extend(l.locals().into_iter().filter(|x| *x != Local::UNIT));
                        }
                    }
                }
                for &bi in &run {
                    seen_before.insert(bi, seen.clone());
                    for l in self.blocks[bi].lines() {
                        for x in l.locals() {
                            if x != Local::UNIT {
                                seen.insert(x);
                            }
                        }
                    }
                }
                once.extend(run);
                first = false;
            }
        }
        self.constant_arrays = if self.optimize {
            self.constant_arrays()
        } else {
            HashSet::new()
        };
        let looping = on_a_loop(&self.blocks);
        let checked = checked_on_card(&self.blocks);
        for bi in 0..self.blocks.len() {
            let name = self.blocks[bi].name;
            self.on_card_now = checked.get(&name).cloned().unwrap_or_default();
            self.in_loop = looping.contains(&name);
            self.panic = match &self.blocks[bi].terminator {
                Terminator::Assert { msg, .. } => Some(check_message(msg)),
                _ => None,
            };
            let l = self.label(name);
            self.define(&l);
            self.sh.pool.reset();
            self.slots.clear();
            self.init_block = once.contains(&bi);
            self.seen = seen_before.get(&bi).cloned().unwrap_or_default();
            self.seen.extend(self.moved_out.iter().copied());
            let block = self.blocks[bi].clone();
            let n = block.statements.len();
            // A comparison feeding a switchInt, or an overflow-checked op feeding its assert, becomes one branch:
            // the bool is never stored. Every block ends in its terminator, so the pair is always the last two lines.
            let fusable = self.optimize && n >= 1 && self.fuse(&block.statements[n - 1], &block.terminator, true)?;
            let end = if fusable { n - 1 } else { n };
            let body: Vec<&Statement> = block.statements[..end]
                .iter()
                .filter(|s| **s != Statement::Nop)
                .collect();
            let at: Vec<usize> = (0..end).filter(|&k| block.statements[k] != Statement::Nop).collect();
            for (i, s) in body.iter().enumerate() {
                self.at = (name, at[i]);
                let after = body
                    .get(i + 1)
                    .copied()
                    .or_else(|| fusable.then(|| &block.statements[n - 1]));
                self.carry = if self.optimize {
                    self.carries(s, after, fusable && i == body.len() - 1)
                } else {
                    None
                };
                self.guard_huge(s);
                self.statement(s).map_err(|e| self.at(e, &source(s)))?;
                if let Statement::Assign(d, _) = s {
                    if let Some(x) = d.base_local() {
                        self.on_card_now.remove(&x);
                    }
                }
                self.carry = None;
                self.wide = false;
            }
            self.init_block = false;
            if fusable {
                let s = &block.statements[n - 1];
                self.guard_huge(s);
                self.fuse(s, &block.terminator, false)
                    .map_err(|e| self.at(e, &source(s)))?;
            } else {
                let t = &block.terminator;
                self.guard_huge_args(t);
                self.terminator(t)
                    .map_err(|e| self.at(e, t.to_string().trim_end_matches(';')))?;
            }
        }
        if returns {
            let exit = Sym::Exit(self.func.clone());
            let mut exit = Line::data_cell(&exit.to_string(), Arg::Num("0".into()));
            exit.note("becomes JMP back to the caller");
            self.code.push(exit);
        }
        let data = self.data()?;
        Ok((std::mem::take(&mut self.code), data))
    }

    /// The function's data lines: its cells, then its arrays, each with its initial value.
    fn data(&mut self) -> Result<Vec<Line>, Fail> {
        // the locals' cells in the order of their numbers, then the cells lowering made up
        let mut cells: Vec<Sym> = self.cells.iter().cloned().collect();
        cells.sort();
        let mut data: Vec<Line> = cells.iter().map(|c| self.data_cell(c)).collect();
        let arrays: Vec<(Sym, usize, Ty)> = self
            .arrays
            .iter()
            .map(|(b, a)| (b.clone(), a.len, a.elem.clone()))
            .collect();
        for (b, a) in self.arrays.iter() {
            let holes: Vec<usize> = (0..a.len).filter(|k| a.hole(*k)).collect();
            if !holes.is_empty() {
                self.sh.holes.push((b.to_string(), holes));
            }
        }
        if let Some(name) = self
            .resources
            .iter()
            .find(|(c, _)| !arrays.iter().any(|(b, ..)| b == *c))
            .map(|(_, n)| n)
        {
            fail!(resource_not_array(name))
        }
        for (base, n, elem) in arrays {
            // a resource's array also gets a table label of its own, resource_NAME: it shows the slot in the
            // listing, and the tape's `# resource NAME ADDRESS LENGTH` is worked out from it (tape::of)
            if let Some(name) = self.resources.get(&base) {
                let mut label = Line::new(Kind::Table {
                    label: format!("resource_{name}"),
                    resource: name.clone(),
                });
                label.note(&format!("resource {name}: the loader fills it"));
                data.push(label);
            }
            let mut first = self.data_cell(&base);
            first.note(&format!("[{elem}; {n}]"));
            data.push(first);
            for k in 1..n {
                let value = self.initial(&Cell::element(&base, k as u32));
                data.push(Line::new(Kind::Dat(None, value)));
            }
        }
        Ok(data)
    }

    /// `name: DAT value`: the cell, with its initial value.
    fn data_cell(&self, c: &Sym) -> Line {
        Line::data_cell(&c.to_string(), self.initial(&Cell::from(c.clone())))
    }

    fn initial(&self, c: &Cell) -> Arg {
        Arg::Num(self.init.get(c).copied().unwrap_or(0).to_string())
    }
}

/// The locals each block knows are on a card when it starts: where the only way in is a passed check `x < k` (or
/// `x <= k`) of a usize against a constant a card holds, `assert(Lt(x, const k))`, or comes from a block that knew
/// it, with nothing setting x on the way. A usize past a card fails that check (lowering tests its sign first), so
/// after it x is 0..k, and needs no test for its sign again (`Lowering::is_huge`).
fn checked_on_card(blocks: &[ir::Block]) -> HashMap<BasicBlock, HashSet<Local>> {
    let preds = ir::predecessors(blocks);
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let sets = |b: &ir::Block, x: Local| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(x)))
            || matches!(&b.terminator, Terminator::Call { dest, .. } if dest.base_local() == Some(x))
    };
    // what a block's passed check shows: x, where the bool it asserts is `x < k` set in the block, x unset after
    let shows = |b: &ir::Block| -> Option<Local> {
        let Terminator::Assert {
            cond: ir::Operand::Copy(p) | ir::Operand::Move(p),
            expected: true,
            ..
        } = &b.terminator
        else {
            return None;
        };
        let v = p.as_local()?;
        let k = b
            .statements
            .iter()
            .rposition(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(v)))?;
        let Statement::Assign(
            _,
            ir::Rvalue::BinaryOp(
                BinOp::Lt | BinOp::Le,
                ir::Operand::Copy(x) | ir::Operand::Move(x),
                ir::Operand::Constant(ir::Const::Int {
                    value,
                    ty: IntTy::Usize,
                }),
            ),
        ) = &b.statements[k]
        else {
            return None;
        };
        let x = x.as_local()?;
        let unset = !b.statements[k + 1..]
            .iter()
            .any(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(x)));
        (unset && (0..=999).contains(value)).then_some(x)
    };
    let mut known: HashMap<BasicBlock, HashSet<Local>> = HashMap::new();
    // (blocks with one way in, from the block before: in the order a walk from that block meets them)
    fn entry(
        i: usize,
        blocks: &[ir::Block],
        preds: &HashMap<BasicBlock, Vec<BasicBlock>>,
        at: &HashMap<BasicBlock, usize>,
        known: &mut HashMap<BasicBlock, HashSet<Local>>,
        out: &dyn Fn(usize, HashSet<Local>) -> HashSet<Local>,
        depth: usize,
    ) -> HashSet<Local> {
        let name = blocks[i].name;
        if let Some(k) = known.get(&name) {
            return k.clone();
        }
        let ins = preds.get(&name).map(Vec::as_slice).unwrap_or_default();
        let facts = match ins {
            [p] if i != 0 && depth < 200 => match at.get(p) {
                Some(&j) if j != i => {
                    let before = entry(j, blocks, preds, at, known, out, depth + 1);
                    out(j, before)
                }
                _ => HashSet::new(),
            },
            _ => HashSet::new(),
        };
        known.insert(name, facts.clone());
        facts
    }
    let out = |j: usize, before: HashSet<Local>| -> HashSet<Local> {
        let b = &blocks[j];
        let mut facts: HashSet<Local> = before.into_iter().filter(|x| !sets(b, *x)).collect();
        facts.extend(shows(b));
        facts
    };
    for i in 0..blocks.len() {
        entry(i, blocks, &preds, &at, &mut known, &out, 0);
    }
    known
}

/// The integer type of an overflow pair `(T, bool)`, which lives in two cells.
fn overflow_pair(ty: &Ty) -> Option<IntTy> {
    match ty {
        Ty::Tuple(ts) if ts.len() == 2 && ts[1] == Ty::Bool => match ts[0] {
            Ty::Int(w @ (IntTy::U8 | IntTy::I8 | IntTy::I16 | IntTy::Usize)) => Some(w),
            _ => None,
        },
        _ => None,
    }
}
