//! The magnitude of v, times 10, over z, signed as v is: rustc divides in each branch of the sign test.
//! A magnitude past 99 overflows a card.
use cardiac::*;

#[inline(never)]
fn scaled(v: i16, z: i16) -> i16 {
    // SAFETY: the fuzzer gives z from 1
    unsafe { assume(z >= 1) };
    let m = if v < 0 { -v } else { v };
    let size = m * 10 / z;
    if v < 0 {
        -size
    } else {
        size
    }
}

fn main() {
    let v = inp();
    let z = inp();
    out(scaled(v, z));
}
