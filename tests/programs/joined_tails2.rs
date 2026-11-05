#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

fn abs(v: i16) -> i16 {
    if v < 0 {
        -v
    } else {
        v
    }
}

/// v's magnitude times z, plus one, signed as v is. Without overflow checks, rustc answers the second sign
/// test from the first, abs's, and copies the multiply and the add into both ways: joined again, they are one call
/// of the multiply routine and one add, and the sign is tested twice.
#[inline(never)]
fn scaled(v: i16, z: i16) -> i16 {
    let size = abs(v) * z + 1;
    if v < 0 {
        -size
    } else {
        size
    }
}

pub fn main() {
    out(scaled(-37, 4));
    out(scaled(4, 4));
}
