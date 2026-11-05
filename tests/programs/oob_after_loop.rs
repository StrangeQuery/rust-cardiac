#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 6] = [1, 7, 1, -17, -13, 2];

/// A bounds check after a loop, the last code before rt_panic: the check's failing edge falls through into
/// the routine, and block layout put the loop's body there instead, so the loop ran again for ever.
pub fn main() {
    let mut x: i16 = 0;
    for _ in 0..3 {
        out(x);
        x += 2;
    }
    out(T[x as usize]);
}
