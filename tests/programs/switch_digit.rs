#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}
unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

/// The multiply by a digit of #34: a match on a digit promised 0 to 9, each arm adding c that many times. Through a
/// jump table, `CLA d`, `ADD` the JMP on the table's first cell, `STO` and run it, and a row of ten JMPs: 5
/// instructions for any digit, where the chain of tests was 2 for each arm passed.
pub fn main() {
    loop {
        let d = inp();
        let c = inp();
        if c == 0 {
            return;
        }
        unsafe { assume(d >= 0 && d <= 9) };
        let s = match d {
            0 => 0,
            1 => c,
            2 => c + c,
            3 => c + c + c,
            4 => c + c + c + c,
            5 => c + c + c + c + c,
            6 => c + c + c + c + c + c,
            7 => c + c + c + c + c + c + c,
            8 => c + c + c + c + c + c + c + c,
            _ => c + c + c + c + c + c + c + c + c,
        };
        out(s);
    }
}
