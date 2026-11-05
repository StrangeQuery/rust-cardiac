#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 4] = [0, -43, 7, 99];

/// A loop that never runs between a loop that does and a bounds check that fails: laid out after the check,
/// the empty loop's body was what its failing edge fell into, so the program ran for ever instead of panicking.
pub fn main() {
    let mut x: i16 = 1;
    for _ in 0..2 {
        x += 1;
    }
    out(x);
    for _ in 0..0 {}
    out(T[(x * 2) as usize]);
}
