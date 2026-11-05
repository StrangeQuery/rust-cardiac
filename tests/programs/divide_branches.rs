#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// The magnitude of v over z, signed as v is: rustc divides in each branch of the sign test, each division with
/// its own check for i16::MIN / -1, which no card needs, and nothing left to read the flag it works out.
#[inline(never)]
fn scaled(v: i16, z: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(z >= 1) };
    let size = if v < 0 { -v } else { v } / z;
    if v < 0 {
        -size
    } else {
        size
    }
}

pub fn main() {
    out(scaled(-37, 4));
    out(scaled(12, 5));
    out(scaled(-5, 1));
}
