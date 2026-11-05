#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `div_euclid` by a constant, which rustc calls at opt-level 1 and s: the quotient rounded so the remainder
/// is never negative, `a / b` one less where `a % b` is below 0 (one more for a divisor below 0).
pub fn main() {
    let mut a: i16 = -23;
    while a < 24 {
        out(a.div_euclid(10));
        out(a.div_euclid(-7));
        a += 9;
    }
}
