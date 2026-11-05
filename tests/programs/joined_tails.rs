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

/// v's magnitude times z, signed as v is. With overflow checks, rustc answers the second sign test from the
/// first, abs's, and copies the multiply and its check into both ways: joined again, they are one call of the
/// multiply routine and one check, and the sign is tested twice.
#[inline(never)]
fn scaled(v: i16, z: i16) -> i16 {
    let size = abs(v) * z;
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
