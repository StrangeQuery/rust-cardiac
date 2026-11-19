#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Arms from 5 up, and runs of values that go to one arm.
#[inline(never)]
fn runs(d: i16) -> i16 {
    match d {
        5 | 6 | 7 => 1,
        8 => 2,
        9 | 10 => 3,
        11 => d,
        12 => d + d,
        13 => 0,
        14 => -d,
        15 => d - 1,
        _ => -1,
    }
}

/// A dense match whose arms start above 0, with runs of values that go to one arm.
pub fn main() {
    let mut d = 2;
    while d < 18 {
        out(runs(d));
        d += 1;
    }
}
