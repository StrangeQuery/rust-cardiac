#![no_std]
#[inline(never)]
pub fn inp() -> i16 {
    0
}
#[inline(never)]
pub fn out(_v: i16) {}

/// A loop made a row (addrow.rs), its count, step and total from the deck: test_compiler.sh checks the total, and
/// the instructions the run takes for each count, one more for each addition. (In a function: a loop main runs once
/// a run stays a loop, its cells not worth loading.)
#[inline(never)]
fn total(d: i16, x: i16, t: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(d >= 0 && d <= 9) };
    let mut t = t;
    for _ in 0..d {
        t += x;
    }
    t
}

pub fn main() {
    let d = inp();
    let x = inp();
    let t = inp();
    out(total(d, x, t));
}
