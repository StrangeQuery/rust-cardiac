#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

fn wrap(v: i16, n: i16) -> i16 {
    if v < 0 {
        v + n
    } else if v >= n {
        v - n
    } else {
        v
    }
}

/// Without overflow checks, a sum that passes a card for a moment and is brought back at once: `u + 110` reaches
/// 1000, and `wrap` takes 900 away from it. The test and the difference straight after the sum read it from the
/// accumulator, all of it, so u comes out 100, as in native Rust, not 0 from the three digits its cell would keep.
pub fn main() {
    let du = core::hint::black_box(-11);
    let dv = core::hint::black_box(88);
    let mut u = core::hint::black_box(450);
    let mut step = dv - du - du;
    for row in -24..4 {
        if row >= 0 {
            out(u);
            step = -dv;
        }
        u = wrap(u + step, 900);
    }
}
