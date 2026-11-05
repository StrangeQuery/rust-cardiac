#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn band(y: i16) -> i16 {
    match y {
        ..6 => 0,
        6..13 => 1,
        13..19 => 2,
        _ => 3,
    }
}

/// A match on disjoint ranges: rustc tests `6 <= y` after `y < 6` failed, which the range pass settles, and
/// the TAC left for `13 <= y` straight after the one for `y < 13` never jumps. Each boundary, both sides.
pub fn main() {
    let mut y = -2;
    while y < 22 {
        out(band(y));
        y += 1;
    }
}
