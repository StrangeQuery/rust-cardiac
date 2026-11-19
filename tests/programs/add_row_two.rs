#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A u8 count masked to 0..=7, two totals, one of them taken away from.
#[inline(never)]
fn two(d: u8, x: i16, y: i16) -> i16 {
    let mut t = 0;
    let mut u = 0;
    for _ in 0..d & 7 {
        t += x;
        u -= y;
    }
    out(u);
    t
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        out(two((v + 20) as u8, 51, -55));
        v += 1;
    }
}
