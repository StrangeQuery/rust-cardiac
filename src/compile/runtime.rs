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
