#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// min, max and clamp of i16, which rustc calls at every opt-level, and core::cmp's min and max.
pub fn main() {
    let xs: [i16; 5] = [-999, -4, 0, 4, 999];
    let mut i = 0usize;
    while i < 5 {
        let x = xs[i];
        let y = xs[4 - i];
        out(x.min(y));
        out(x.max(y));
        out(x.clamp(-5, 5));
        out(core::cmp::min(x, 2));
        out(core::cmp::max(x, -2));
        i += 1;
    }
}
