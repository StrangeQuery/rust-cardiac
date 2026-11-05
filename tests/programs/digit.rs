#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
/// The digit() intrinsic: on CARDIAC a shift instruction built for k and run.
#[inline(never)]
pub fn digit(v: i16, k: i16) -> i16 { if k == 0 { v / 100 } else if k == 1 { v / 10 % 10 } else { v % 10 } }

pub fn main() {
    let xs: [i16; 3] = [907, 12, 345];
    let mut j = 0usize;
    while j < 3 {
        let mut k = 0;
        while k < 3 { out(digit(xs[j], k)); k += 1; }
        j += 1;
    }
}
