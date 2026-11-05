#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A counter that is the function's result (`_0`): only set, stepped and compared with 16, like a counter that only
/// counts, but the caller reads it, so it must keep its value, not count to 0 (found in a larger program's pclk).
#[inline(never)]
fn clock(frame: i16, every: i16) -> i16 {
    let mut left = frame;
    let mut ticks = 0;
    while left >= every {
        left -= every;
        ticks += 1;
        if ticks >= 16 {
            ticks = 0;
        }
    }
    ticks
}

pub fn main() {
    out(clock(0, 2));
    out(clock(40, 2));
    out(clock(40, 3));
}
