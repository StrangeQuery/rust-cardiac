#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A C-like enum beside a `for` loop over a range: the enum's `discriminant()` is its number, and the loop's
/// `Option`'s is left for the for-loop pass, which lowers the loop by it (at opt-level 1 it was refused).
#[derive(Clone, Copy)]
enum Ek0 {
    A,
    B,
    C,
}

#[inline(never)]
fn ek0(v: i16) -> Ek0 {
    if v < -31 {
        Ek0::A
    } else if v < 15 {
        Ek0::B
    } else {
        Ek0::C
    }
}

pub fn main() {
    for j in 0..3 {
        out(j);
    }
    out(ek0(0) as i16);
    out(ek0(20) as i16);
}
