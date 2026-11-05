#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// What rustc calls at opt-level 1 and s and inlines at 2: wrapping_add, wrapping_sub and wrapping_mul (on
/// values a card holds, the plain operation), rem_euclid by a constant, abs_diff and saturating_sub of usize.
pub fn main() {
    let xs: [i16; 5] = [-23, -7, 0, 5, 23];
    let mut i = 0usize;
    while i < 5 {
        let x = xs[i];
        out(x.wrapping_add(xs[4 - i]));
        out(x.wrapping_sub(3));
        out(x.wrapping_mul(4));
        out(x.rem_euclid(10));
        out(x.rem_euclid(-100));
        out(i.abs_diff(2) as i16);
        out(i.saturating_sub(2) as i16);
        out(3usize.saturating_sub(i) as i16);
        i += 1;
    }
}
