#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// abs, which rustc calls at opt-level 1 and s: a sign test and 0 - x. The range pass knows the result is 0
/// or more, so `t[(x / 100).abs() as usize]` needs no test that the index is not negative, nor one of its bounds.
pub fn main() {
    let xs: [i16; 6] = [-999, -7, -1, 0, 3, 999];
    let t: [i16; 10] = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19];
    let mut i = 0usize;
    while i < 6 {
        let x = xs[i];
        out(x.abs());
        out(t[(x / 100).abs() as usize]);
        i += 1;
    }
}
