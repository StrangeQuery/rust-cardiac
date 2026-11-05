//! The cells and labels lowering names, typed. A symbol says what a cell is for; it becomes text only as an
//! instruction is emitted, as the name the listing gives it.
use std::fmt;

use crate::compile::ir::{BasicBlock, Local};
use crate::compile::machine::Arg;

/// What a self-modifying cell does with the address it is built on: read it (CLA), write it (STO) or punch it
/// (OUT); or go there, into a row of identical instructions, run from that cell on (TAC, JMP).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Access {
    Read,
    Write,
    Out,
    Branch,
    Jump,
}

impl Access {
    /// The instruction on cell 00: the one on cell n is this plus n.
    pub fn opcode(self) -> i64 {
        match self {
            Access::Read => 100,
            Access::Write => 600,
            Access::Out => 500,
            Access::Branch => 300,
            Access::Jump => 800,
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Access::Read => "rd",
            Access::Write => "wr",
            Access::Out => "out",
            Access::Branch => "tac",
            Access::Jump => "jmp",
        }
    }
}

/// A cell or a label, as lowering names it. Symbols order by kind, in the order below, then by what they hold: a
/// function's locals by number, each whole local before its fields.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sym {
    /// A function's local, or one field of it: `main_3`, `add_3_1`.
    Local {
        func: String,
        local: Local,
        field: Option<u32>,
    },
    /// A cell of the shared scratch pool: `t0`.
    Scratch(usize),
    /// A constant: `k5`, `kn3`.
    Const(i64),
    /// A constant holding a cell's address: `a_main_3`, `a_main_xs_2`.
    Address(Box<Cell>),
    /// A constant holding an instruction on a cell: `rd_main_xs`, `wr_main_xs_0`, `out_main_xs`.
    Instr(Access, Box<Cell>),
    /// A basic block's label: `main_bb3`.
    Block(String, BasicBlock),
    /// A label or a cell lowering makes up, numbered within its function: `main_rc4`, `main_sm7`.
    Fresh { func: String, stem: &'static str, n: usize },
    /// Where a function starts: `add_entry`.
    Entry(String),
    /// The cell a function keeps its return instruction in, and jumps to: `add_exit`.
    Exit(String),
    /// A shared routine's label or cell: `rt_panic`, `rt_mul_a`.
    Runtime(&'static str),
}

impl Sym {
    /// A function's local, the whole of it.
    pub fn local(func: &str, local: Local) -> Sym {
        Sym::Local {
            func: func.to_string(),
            local,
            field: None,
        }
    }

    /// What a constant cell holds, for the data section: the number, the address, or the instruction.
    pub fn contents(&self) -> Option<Arg> {
        match self {
            Sym::Const(v) => Some(Arg::Num(v.to_string())),
            Sym::Address(cell) => Some(Arg::from((**cell).clone())),
            Sym::Instr(access, cell) => {
                let on = access.opcode() as u32 + cell.offset.unwrap_or(0);
                Some(Arg::Sym(cell.sym.to_string(), Some(on)))
            }
            _ => None,
        }
    }
}

/// A symbol's cell, or the one `offset` cells after it: `main_3`, `main_xs+2`. An offset of 0 is not the same as
/// none: it prints, `main_xs+0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    pub sym: Sym,
    pub offset: Option<u32>,
}

impl Cell {
    /// Element k of the array at `base`: for 0, the base itself.
    pub fn element(base: &Sym, k: u32) -> Cell {
        Cell {
            sym: base.clone(),
            offset: (k > 0).then_some(k),
        }
    }

    /// The cell as part of a constant's name: `main_xs_2` for `main_xs+2`.
    fn mangled(&self) -> String {
        match self.offset {
            None => self.sym.to_string(),
            Some(k) => format!("{}_{k}", self.sym),
        }
    }
}

impl From<Sym> for Cell {
    fn from(sym: Sym) -> Cell {
        Cell { sym, offset: None }
    }
}

/// What an instruction names: a cell by symbol, or a cell by its number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Cell(Cell),
    Fixed(u8),
}

impl Operand {
    /// Cell 00, wired to 001: a free constant one (the trick from a hand-written FizzBuzz). It is
    /// also where the loader starts.
    pub const CELL_00: Operand = Operand::Fixed(0);
    /// Cell 99, where JMP leaves its return instruction.
    pub const CELL_99: Operand = Operand::Fixed(99);
}

impl From<Cell> for Operand {
    fn from(cell: Cell) -> Operand {
        Operand::Cell(cell)
    }
}

impl From<Sym> for Operand {
    fn from(sym: Sym) -> Operand {
        Operand::Cell(Cell::from(sym))
    }
}

impl From<Sym> for Arg {
    fn from(sym: Sym) -> Arg {
        Arg::Sym(sym.to_string(), None)
    }
}

impl From<Cell> for Arg {
    fn from(cell: Cell) -> Arg {
        Arg::Sym(cell.sym.to_string(), cell.offset)
    }
}

impl From<Operand> for Arg {
    fn from(o: Operand) -> Arg {
        match o {
            Operand::Cell(cell) => Arg::from(cell),
            Operand::Fixed(n) => Arg::Num(format!("{n:02}")),
        }
    }
}

/// SFT's operand: shift left `left` digits, then right `right`.
pub fn shift(left: i64, right: i64) -> Arg {
    Arg::Num(format!("{left}{right}"))
}

impl fmt::Display for Sym {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Sym::Local {
                func,
                local,
                field: None,
            } => write!(f, "{func}{local}"),
            Sym::Local {
                func,
                local,
                field: Some(k),
            } => write!(f, "{func}{local}_{k}"),
            Sym::Scratch(n) => write!(f, "t{n}"),
            Sym::Const(v) if *v < 0 => write!(f, "kn{}", -v),
            Sym::Const(v) => write!(f, "k{v}"),
            Sym::Address(cell) => write!(f, "a_{}", cell.mangled()),
            Sym::Instr(access, cell) => write!(f, "{}_{}", access.stem(), cell.mangled()),
            Sym::Block(func, bb) => write!(f, "{func}_{bb}"),
            Sym::Fresh { func, stem, n } => write!(f, "{func}_{stem}{n}"),
            Sym::Entry(func) => write!(f, "{func}_entry"),
            Sym::Exit(func) => write!(f, "{func}_exit"),
            Sym::Runtime(name) => f.write_str(name),
        }
    }
}
