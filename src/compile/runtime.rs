//! The runtime: the small shared routines a program calls for what CARDIAC has no instruction for (multiply,
//! divide, the i16 check, the panic), each emitted once into a program that needs it, and the data section that
//! follows the code (the scratch cells, the routines' cells, the constants).
//!
//! A routine is called by `JMP rt_x_entry`: the JMP leaves its return jump in cell 99, and the routine copies it
//! into its last cell, `rt_x_exit`, before anything else can overwrite it.
// the routines are written with asm!, which builds a listing a push at a time, an instruction a line, as it reads
#![allow(clippy::vec_init_then_push)]
use super::lower::{Access, Cell, Pool, Sym, PANIC};
use super::machine::{Arg, Line, Op};

/// The opcode of a mnemonic, at compile time.
macro_rules! op {
    (INP) => {
        Op::Inp
    };
    (CLA) => {
        Op::Cla
    };
    (ADD) => {
        Op::Add
    };
    (TAC) => {
        Op::Tac
    };
    (SFT) => {
        Op::Sft
    };
    (OUT) => {
        Op::Out
    };
    (STO) => {
        Op::Sto
    };
    (SUB) => {
        Op::Sub
    };
    (JMP) => {
        Op::Jmp
    };
    (HRS) => {
        Op::Hrs
    };
}

/// Assembly in the machine IR: `name:` is a label, `CLA rt_mul_b` an instruction on a symbol, `SUB 00` one on a
/// cell by number.
macro_rules! asm {
    (@ $lines:ident) => {};
    (@ $lines:ident $label:ident : $($rest:tt)*) => {
        $lines.push(Line::label(stringify!($label)));
        asm!(@ $lines $($rest)*);
    };
    (@ $lines:ident $op:ident $cell:literal $($rest:tt)*) => {
        $lines.push(Line::instr(op!($op), Arg::Num(stringify!($cell).to_string())));
        asm!(@ $lines $($rest)*);
    };
    (@ $lines:ident $op:ident $symbol:ident $($rest:tt)*) => {
        $lines.push(Line::instr(op!($op), Arg::sym(stringify!($symbol))));
        asm!(@ $lines $($rest)*);
    };
    ($($items:tt)*) => {{
        let mut lines: Vec<Line> = Vec::new();
        asm!(@ lines $($items)*);
        lines
    }};
}

/// A routine's last cell, `rt_x_exit`: the return jump is stored there on entry.
fn exit_cell(name: &str) -> Line {
    let mut line = Line::data_cell(name, Arg::Num("0".into()));
    line.note("becomes JMP back to the caller");
    line
}

/// a * b by repeated addition, |b| times (put the smaller factor second); the product in the accumulator.
/// Checked: every partial product must stay a card, else panic (the partials only grow).
fn mul(checked: bool) -> Vec<Line> {
    let mut r = asm![
        rt_mul_entry:
            CLA 99
            STO rt_mul_exit
            CLA k0
            STO rt_mul_r
            CLA rt_mul_b
            TAC rt_mul_neg
        rt_mul_loop:
            CLA rt_mul_b
            SUB 00
            TAC rt_mul_done
            STO rt_mul_b
            CLA rt_mul_r
            ADD rt_mul_a
            STO rt_mul_r
    ];
    if checked {
        r.extend(asm![
            ADD k999
            TAC rt_panic
            SUB k999
            SUB k999
            SUB 00
            TAC rt_mul_loop
            JMP rt_panic
        ]);
    } else {
        r.extend(asm![JMP rt_mul_loop]);
    }
    r.extend(asm![
        rt_mul_neg:
            CLA k0
            SUB rt_mul_b
            STO rt_mul_b
            CLA k0
            SUB rt_mul_a
            STO rt_mul_a
            JMP rt_mul_loop
        rt_mul_done:
            CLA rt_mul_r
    ]);
    r.push(exit_cell("rt_mul_exit"));
    r
}

/// a / b truncated toward zero into the accumulator (and a % b, with the dividend's sign, into rt_div_r when the
/// program asks for one): on |a| and |b| by repeated subtraction, the quotient counted in steps of rt_div_d, which
/// is -1 when exactly one of them was negative. rt_div_rn is 1 when the dividend was negative.
fn div(rem: bool) -> Vec<Line> {
    let mut r = asm![
        rt_div_entry:
            CLA 99
            STO rt_div_exit
            CLA k0
            STO rt_div_q
    ];
    if rem {
        r.extend(asm![STO rt_div_rn]);
    }
    r.extend(asm![
            CLA 00
            STO rt_div_d
            CLA rt_div_b
            TAC rt_div_nb
        rt_div_1:
            CLA rt_div_a
            TAC rt_div_na
        rt_div_lp:
            SUB rt_div_b
            TAC rt_div_end
            STO rt_div_a
            CLA rt_div_q
            ADD rt_div_d
            STO rt_div_q
            CLA rt_div_a
            JMP rt_div_lp
        rt_div_end:
    ]);
    if rem {
        r.extend(asm![
                ADD rt_div_b
                STO rt_div_r
                CLA rt_div_rn
                SUB 00
                TAC rt_div_rp
                CLA k0
                SUB rt_div_r
                STO rt_div_r
            rt_div_rp:
        ]);
    }
    r.extend(asm![
            CLA rt_div_q
            JMP rt_div_exit
        rt_div_nb:
            CLA k0
            SUB rt_div_b
            STO rt_div_b
            CLA k0
            SUB 00
            STO rt_div_d
            JMP rt_div_1
        rt_div_na:
            CLA k0
            SUB rt_div_a
            STO rt_div_a
    ]);
    if rem {
        r.extend(asm![
            CLA 00
            STO rt_div_rn
        ]);
    }
    r.extend(asm![
            CLA k0
            SUB rt_div_d
            STO rt_div_d
            CLA rt_div_a
            JMP rt_div_lp
    ]);
    r.push(exit_cell("rt_div_exit"));
    r
}

/// The i16 check, shared: is the accumulator still a card, -999..999? Back if so, else panic. The value is tested
/// in the accumulator, whole (a cell would keep only three digits of it), and the way back is cell 99 itself: the
/// JMP that called here left the return jump there, and nothing here jumps before it is used.
fn chk() -> Vec<Line> {
    let mut r = asm![
        rt_chk_entry:
            ADD k999
            TAC rt_panic
            SUB k999
            SUB k999
            SUB 00
    ];
    r.push(Line::commented(Op::Tac, Arg::Num("99".into()), "back to the caller"));
    r.extend(asm![JMP rt_panic]);
    r
}

/// A cell of a routine's code that the routine itself builds an instruction in before it runs it.
fn slot(name: &str, note: &str) -> Line {
    let mut line = Line::data_cell(name, Arg::Num("0".into()));
    line.note(note);
    line
}

/// The instruction a row is entered by, less the digit, on the row's end (`DAT rt_skip_end+300`, TAC
/// rt_skip_end): kept in the routine's code, after its last instruction, as the data section is assembled on its
/// own and can't name a label in the code.
fn on_end(access: Access, end: &'static str) -> Line {
    let c = Sym::Instr(access, Box::new(Cell::from(Sym::Runtime(end))));
    Line::data_cell(&c.to_string(), c.contents().expect("an instruction constant"))
}

/// How many identical instructions a row holds: `skip_cards()` runs whole rows, one for each ten.
pub const ROW: usize = 10;

/// `skip_cards(n)`, n (0..=999) in the accumulator: a computed jump into a row of ten `INP 00`, the units first,
/// then the whole row once for each ten. The jump is a TAC, built as `TAC rt_skip_end - u` (`tac_rt_skip_end` less
/// the units), taken because the accumulator holds -T-1 for T tens: below 0, and one nearer 0 after each pass, so
/// the TAC at the end runs the row again until it is 0. `SUB 00` and `TAC 99` then go back to the caller: nothing
/// here is a JMP, so cell 99 still holds the caller's way back. 15 instructions, one a card and 2 a ten.
fn skip() -> Vec<Line> {
    let mut r = asm![
        rt_skip_entry:
            STO rt_skip_go
            SFT 01
            STO rt_skip_t
            SFT 10
            SUB rt_skip_go
            ADD tac_rt_skip_end
            STO rt_skip_go
            CLA kn1
            SUB rt_skip_t
    ];
    r.push(slot("rt_skip_go", "becomes TAC into the row: the units"));
    r.push(Line::label("rt_skip_row"));
    for _ in 0..ROW {
        r.push(Line::instr(Op::Inp, Arg::Num("00".into())));
    }
    r.extend(asm![
        rt_skip_end:
            ADD 00
            TAC rt_skip_row
            SUB 00
    ]);
    r.push(Line::commented(Op::Tac, Arg::Num("99".into()), "back to the caller"));
    r.push(on_end(Access::Branch, "rt_skip_end"));
    r
}

/// `add_times(c, n)`, c in rt_times_c and n (0..=99) in the accumulator: n c as (T c) 10 + u c, for T tens and u
/// units, each part a computed jump into a row of nine `ADD rt_times_c`, built as `JMP rt_times_end - digit`. The
/// row ends in `TAC 99`, back to the cell after the JMP that came in, so the accumulator must be below 0 there: the
/// tens start from -100 (T c is within -99..=99 when n c fits on a card), and the shift that makes T c - 100 ten
/// times as much leaves the units -1000 to start from. Adding 1000 back gives n c. 24 instructions and one for
/// each addition.
fn times() -> Vec<Line> {
    let mut r = asm![
        rt_times_entry:
            STO rt_times_go2
            SFT 01
            STO rt_times_go1
            SFT 10
            SUB rt_times_go2
            ADD jmp_rt_times_end
            STO rt_times_go2
            CLA 99
            STO rt_times_exit
            CLA jmp_rt_times_end
            SUB rt_times_go1
            STO rt_times_go1
            CLA kn100
    ];
    r.push(slot("rt_times_go1", "becomes JMP into the row: the tens"));
    r.extend(asm![SFT 10]);
    r.push(slot("rt_times_go2", "becomes JMP into the row: the units"));
    r.extend(asm![
            SUB kn500
            SUB kn500
    ]);
    r.push(exit_cell("rt_times_exit"));
    r.push(Line::label("rt_times_row"));
    for _ in 0..ROW - 1 {
        r.push(Line::instr(Op::Add, Arg::sym("rt_times_c")));
    }
    r.push(Line::label("rt_times_end"));
    r.push(Line::commented(
        Op::Tac,
        Arg::Num("99".into()),
        "back to the cell after the jump in",
    ));
    // past a card: no way back through 99 (and n c is undefined)
    r.extend(asm![JMP rt_times_exit]);
    r.push(on_end(Access::Jump, "rt_times_end"));
    r
}

/// `&`, `|` and `^` between two integers: a routine each, put in for the program that uses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bitwise {
    And,
    Or,
    Xor,
}

impl Bitwise {
    /// The bit it makes of two bits.
    fn bit(self, a: bool, b: bool) -> bool {
        match self {
            Bitwise::And => a && b,
            Bitwise::Or => a || b,
            Bitwise::Xor => a != b,
        }
    }
}

/// A bitwise routine: for any two values on a card, or for two u8s, which need neither a sign nor a check and have
/// eight bits, not ten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitRoutine {
    pub op: Bitwise,
    pub byte: bool,
}

impl BitRoutine {
    const ALL: [BitRoutine; 6] = [
        BitRoutine {
            op: Bitwise::And,
            byte: false,
        },
        BitRoutine {
            op: Bitwise::Or,
            byte: false,
        },
        BitRoutine {
            op: Bitwise::Xor,
            byte: false,
        },
        BitRoutine {
            op: Bitwise::And,
            byte: true,
        },
        BitRoutine {
            op: Bitwise::Or,
            byte: true,
        },
        BitRoutine {
            op: Bitwise::Xor,
            byte: true,
        },
    ];

    /// The routine's entry, and the cells its two operands are passed in.
    pub fn cells(self) -> [&'static str; 3] {
        match (self.op, self.byte) {
            (Bitwise::And, false) => ["rt_and_entry", "rt_and_a", "rt_and_b"],
            (Bitwise::Or, false) => ["rt_or_entry", "rt_or_a", "rt_or_b"],
            (Bitwise::Xor, false) => ["rt_xor_entry", "rt_xor_a", "rt_xor_b"],
            (Bitwise::And, true) => ["rt_and8_entry", "rt_and8_a", "rt_and8_b"],
            (Bitwise::Or, true) => ["rt_or8_entry", "rt_or8_a", "rt_or8_b"],
            (Bitwise::Xor, true) => ["rt_xor8_entry", "rt_xor8_a", "rt_xor8_b"],
        }
    }

    /// One of its cells or labels: `rt_and_r`.
    fn name(self, part: &str) -> String {
        let op = match self.op {
            Bitwise::And => "and",
            Bitwise::Or => "or",
            Bitwise::Xor => "xor",
        };
        format!("rt_{op}{}_{part}", if self.byte { "8" } else { "" })
    }
}

/// a op b, for a in rt_op_a and b in rt_op_b, as two's complement has it; the result into the accumulator. A card
/// is 11 bits of two's complement: the sign, which stands for -1024, and ten more. Each operand is held as the bits
/// of it not yet read, the low ten less 512 (a + 512 below 0, a - 512 from 0), so that the next bit is whether that
/// is 0 or more, and reading it moves the rest up (2s - 512 after a 1, 2s + 512 after a 0). The result is built as
/// the bits are read: 0 or -1 for the two signs, then 2r and the two bits' bit, ten times. Only the last can be past
/// a card (-513 & -512 is -1024, 512 | 511 is 1023): it panics there. Two u8s are eight bits, less 128, and no sign.
fn bitwise(routine: BitRoutine) -> Vec<Line> {
    let (op, byte) = (routine.op, routine.byte);
    let name = |part: &str| routine.name(part);
    let (a, b, r, n) = (name("a"), name("b"), name("r"), name("n"));
    let half = if byte { "k128" } else { "k512" };
    let i = |o: Op, arg: &str| {
        let arg = if arg.bytes().all(|c| c.is_ascii_digit()) {
            Arg::Num(arg.into())
        } else {
            Arg::sym(arg)
        };
        Line::instr(o, arg)
    };
    let label = |part: &str| Line::label(&name(part));
    let leaf = |bit: bool| if bit { "one" } else { "zero" };
    let mut code = vec![
        label("entry"),
        i(Op::Cla, "99"),
        i(Op::Sto, &name("exit")),
        i(Op::Cla, if byte { "k7" } else { "k9" }),
        i(Op::Sto, &n),
        i(Op::Cla, "k0"),
        i(Op::Sto, &r),
    ];
    if byte {
        code.extend([
            i(Op::Cla, &a),
            i(Op::Sub, half),
            i(Op::Sto, &a),
            i(Op::Cla, &b),
            i(Op::Sub, half),
            i(Op::Sto, &b),
        ]);
    } else {
        // r is -1 where b's sign bit is 1; given a's, it becomes minus the result's
        let sign_a = |sa: bool| match (op.bit(sa, false), op.bit(sa, true)) {
            (false, true) => vec![],
            (false, false) => vec![i(Op::Cla, "k0"), i(Op::Sto, &r)],
            (true, true) => vec![i(Op::Cla, "kn1"), i(Op::Sto, &r)],
            (true, false) => vec![i(Op::Cla, "kn1"), i(Op::Sub, &r), i(Op::Sto, &r)],
        };
        code.extend([
            i(Op::Cla, &b),
            i(Op::Tac, &name("bn")),
            i(Op::Sub, half),
            i(Op::Sto, &b),
            i(Op::Jmp, &name("as")),
            label("bn"),
            i(Op::Add, half),
            i(Op::Sto, &b),
            i(Op::Cla, "kn1"),
            i(Op::Sto, &r),
            label("as"),
            i(Op::Cla, &a),
            i(Op::Tac, &name("an")),
            i(Op::Sub, half),
            i(Op::Sto, &a),
        ]);
        code.extend(sign_a(false));
        code.extend([i(Op::Jmp, &name("loop")), label("an"), i(Op::Add, half), i(Op::Sto, &a)]);
        code.extend(sign_a(true));
    }
    // a bit of each: a's by the path taken, b's in the leaf, which moves b on and doubles r, adding 1 where the
    // result's bit is 1 (a 1 of b is first taken down twice as far, so that the leaf moves it on as it does a 0)
    code.extend([
        label("loop"),
        i(Op::Cla, &a),
        i(Op::Tac, &name("a0")),
        i(Op::Add, &a),
        i(Op::Sub, half),
        i(Op::Sto, &a),
        i(Op::Cla, &b),
        i(Op::Tac, &name(leaf(op.bit(true, false)))),
        i(Op::Sub, half),
        i(Op::Sub, half),
        i(Op::Jmp, &name(leaf(op.bit(true, true)))),
        label("a0"),
        i(Op::Add, &a),
        i(Op::Add, half),
        i(Op::Sto, &a),
        i(Op::Cla, &b),
        i(Op::Tac, &name(leaf(op.bit(false, false)))),
        i(Op::Sub, half),
        i(Op::Sub, half),
    ]);
    // the leaf a0's 1 falls into, then the other, which falls into the store
    let first = op.bit(false, true);
    for bit in [first, !first] {
        code.extend([
            label(leaf(bit)),
            i(Op::Add, &b),
            i(Op::Add, half),
            i(Op::Sto, &b),
            i(Op::Cla, &r),
            i(Op::Add, &r),
        ]);
        if bit {
            code.push(i(Op::Add, "00"));
        }
        if bit == first {
            code.push(i(Op::Jmp, &name("st")));
        }
    }
    code.extend([label("st"), i(Op::Sto, &r)]);
    if !byte {
        // r read back: what the cell kept is all of it only on a card
        code.extend([
            i(Op::Sub, &r),
            i(Op::Tac, "rt_panic"),
            i(Op::Sub, "00"),
            i(Op::Tac, &name("ok")),
            i(Op::Jmp, "rt_panic"),
            label("ok"),
        ]);
    }
    code.extend([
        i(Op::Cla, &n),
        i(Op::Sub, "00"),
        i(Op::Sto, &n),
        i(Op::Tac, &name("done")),
        i(Op::Jmp, &name("loop")),
        label("done"),
        i(Op::Cla, &r),
        exit_cell(&name("exit")),
    ]);
    code
}

/// a >> n, a on a card (in rt_shr_a) and n from 0 to 63 (in the accumulator): n halvings, each rounding down. A
/// halving, 5z and a decimal shift right, rounds toward 0, so a value below 0 is one more first and one less after:
/// for z below 0, z halved down is one less than z + 1 halved toward 0, halving after halving.
fn shr() -> Vec<Line> {
    asm![
        rt_shr_entry:
            STO rt_shr_n
            CLA 99
            STO rt_shr_exit
            CLA rt_shr_a
            TAC rt_shr_up
            JMP rt_shr_loop
        rt_shr_up:
            ADD 00
        rt_shr_loop:
            STO rt_shr_z
            CLA rt_shr_n
            SUB 00
            TAC rt_shr_done
            STO rt_shr_n
            CLA rt_shr_z
            ADD rt_shr_z
            ADD rt_shr_z
            ADD rt_shr_z
            ADD rt_shr_z
            SFT 01
            JMP rt_shr_loop
        rt_shr_done:
            CLA rt_shr_a
            TAC rt_shr_down
            CLA rt_shr_z
            JMP rt_shr_exit
        rt_shr_down:
            CLA rt_shr_z
            SUB 00
    ]
    .into_iter()
    .chain([exit_cell("rt_shr_exit")])
    .collect()
}

/// a << n of an i16, a on a card (in rt_shl_a) and n from 0 to 15 (in the accumulator), kept in 16 bits as Rust
/// keeps it (513 << 7 is 128), and panicking past a card: a's low 16 - n bits read as a number of that many bits,
/// doubled n times. That number is a brought into -h..h - 1, h = 2^(15 - n), by adding or taking away 2h; for n
/// below 6 it is a, as a card has fewer bits.
fn shl() -> Vec<Line> {
    let mut r = asm![
        rt_shl_entry:
            STO rt_shl_n
            CLA 99
            STO rt_shl_exit
            CLA rt_shl_n
            SUB k6
            TAC rt_shl_dbl
            CLA k15
            SUB rt_shl_n
            STO rt_shl_i
            CLA 00
        rt_shl_h:
            STO rt_shl_hh
            CLA rt_shl_i
            SUB 00
            TAC rt_shl_down
            STO rt_shl_i
            CLA rt_shl_hh
            ADD rt_shl_hh
            JMP rt_shl_h
        rt_shl_down:
            CLA rt_shl_a
            SUB rt_shl_hh
            TAC rt_shl_lo
            SUB rt_shl_hh
            STO rt_shl_a
            JMP rt_shl_down
        rt_shl_lo:
            ADD rt_shl_hh
            ADD rt_shl_hh
            TAC rt_shl_up
            JMP rt_shl_dbl
        rt_shl_up:
            ADD rt_shl_hh
            STO rt_shl_a
            ADD rt_shl_hh
            TAC rt_shl_up
        rt_shl_dbl:
            CLA rt_shl_n
            SUB 00
            TAC rt_shl_done
            STO rt_shl_n
            CLA rt_shl_a
            ADD rt_shl_a
            STO rt_shl_a
            SUB rt_shl_a
            TAC rt_panic
            SUB 00
            TAC rt_shl_dbl
            JMP rt_panic
        rt_shl_done:
            CLA rt_shl_a
    ];
    r.push(exit_cell("rt_shl_exit"));
    r
}

/// a << n of a u8, a in rt_shl8_a and n from 0 to 7 in the accumulator: n doublings, each mod 256, as a u8 keeps it.
fn shl8() -> Vec<Line> {
    let mut r = asm![
        rt_shl8_entry:
            STO rt_shl8_n
            CLA 99
            STO rt_shl8_exit
            CLA rt_shl8_a
        rt_shl8_loop:
            STO rt_shl8_a
            CLA rt_shl8_n
            SUB 00
            TAC rt_shl8_done
            STO rt_shl8_n
            CLA rt_shl8_a
            ADD rt_shl8_a
            SUB k256
            TAC rt_shl8_lo
            JMP rt_shl8_loop
        rt_shl8_lo:
            ADD k256
            JMP rt_shl8_loop
        rt_shl8_done:
            CLA rt_shl8_a
    ];
    r.push(exit_cell("rt_shl8_exit"));
    r
}

/// Why a runtime routine panics: a value too big for a card, where native Rust's i16 would carry on.
pub const NO_CARD: &str = "the value doesn't fit on a card (-999 to 999)";

/// The panic routine punches -999, kept in a constant cell, and halts.
fn panic() -> Vec<Line> {
    let punch = Line::commented(
        Op::Out,
        Arg::sym(&format!("kn{}", -PANIC)),
        "a value did not fit, or a check failed",
    );
    let mut r = asm![rt_panic:];
    r.push(punch);
    r.extend(asm![HRS 0]);
    r
}

/// The routines the program uses, to follow its code; and its data section: a cell for each scratch slot of the
/// pool, the routines' cells, then the constants (with the ones the routines need added to `consts`).
pub fn runtime(pool: &Pool, consts: &[Sym]) -> (Vec<Line>, Vec<Line>) {
    let (mut code, mut data) = (vec![], vec![]);
    for k in 0..pool.size {
        let mut scratch = Line::data_cell(&format!("t{k}"), Arg::Num("0".into()));
        scratch.note("scratch");
        data.push(scratch);
    }
    let zero = |name: &str| Line::data_cell(name, Arg::Num("0".into()));
    let mut needed: Vec<i64> = vec![];
    let mut panics = pool.panic;
    if pool.mul {
        code.extend(mul(pool.mul_checked));
        data.extend(["rt_mul_a", "rt_mul_b", "rt_mul_r"].map(zero));
        needed.push(0);
        if pool.mul_checked {
            needed.push(999);
            panics = true;
        }
    }
    if pool.div {
        code.extend(div(pool.divr));
        data.extend(["rt_div_a", "rt_div_b", "rt_div_q", "rt_div_d"].map(zero));
        if pool.divr {
            data.extend(["rt_div_r", "rt_div_rn"].map(zero));
        }
        needed.push(0);
    }
    if pool.chk {
        code.extend(chk());
        needed.push(999);
    }
    if pool.skip {
        code.extend(skip());
        data.push(zero("rt_skip_t"));
        needed.push(-1);
    }
    if pool.times {
        code.extend(times());
        data.push(zero("rt_times_c"));
        needed.extend([-100, -500]);
    }
    for routine in BitRoutine::ALL.into_iter().filter(|r| pool.bitwise.contains(r)) {
        code.extend(bitwise(routine));
        let [_, a, b] = routine.cells();
        data.extend([a, b].map(zero));
        data.extend(["r", "n"].map(|part| zero(&routine.name(part))));
        if routine.byte {
            needed.extend([0, 7, 128]);
        } else {
            needed.extend([0, 9, 512, -1]);
            panics = true;
        }
    }
    if pool.shr {
        code.extend(shr());
        data.extend(["rt_shr_a", "rt_shr_n", "rt_shr_z"].map(zero));
    }
    if pool.shl {
        code.extend(shl());
        data.extend(["rt_shl_a", "rt_shl_n", "rt_shl_i", "rt_shl_hh"].map(zero));
        needed.extend([6, 15]);
        panics = true;
    }
    if pool.shl8 {
        code.extend(shl8());
        data.extend(["rt_shl8_a", "rt_shl8_n"].map(zero));
        needed.push(256);
    }
    // the routines' own checks: the value is no card (rt_mul's product, rt_chk's total), a panic of CARDIAC's
    for branch in code.iter_mut().filter(|line| line.to_panic()) {
        branch.panic = Some(NO_CARD.into());
    }
    if panics {
        code.extend(panic());
        needed.push(PANIC);
    }
    let mut consts = consts.to_vec();
    for v in needed {
        if !consts.contains(&Sym::Const(v)) {
            consts.push(Sym::Const(v));
        }
    }
    data.extend(
        consts
            .iter()
            .map(|c| Line::data_cell(&c.to_string(), c.contents().expect("a constant cell"))),
    );
    (code, data)
}
