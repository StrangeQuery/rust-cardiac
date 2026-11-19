#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

/// A digit's arms, the digit promised 0 to 9: no test of the range, a jump into a row of ten.
#[inline(never)]
fn arms(d: i16, c: i16) -> i16 {
    unsafe { assume(d >= 0 && d <= 9) };
    match d {
        0 => 0,
        1 => c,
        2 => c + c,
        3 => c + c + c,
        4 => -c,
        5 => c - 1,
        6 => c + 1,
        7 => 1 - c,
        8 => -c - c,
        _ => c + 2,
    }
}

/// A dense match through a jump table (`lower::branches`), its range promised.
pub fn main() {
    let mut d = 0;
    while d < 10 {
        out(arms(d, 3));
        d += 1;
    }
}
