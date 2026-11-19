#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn digit(v: i16, k: i16) -> i16 { if k == 0 { v / 100 } else if k == 1 { v / 10 % 10 } else { v % 10 } }

/// What doesn't change worked out once: a value's tens and units added up apart, a digit's times.
#[inline(never)]
fn split(v: i16, w: i16) -> i16 {
    let mut tens = 0;
    let mut units = 0;
    for _ in 0..digit(w, 2) {
        tens += v / 10;
        units += v % 10;
    }
    tens - units
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        out(split(-165, v));
        v += 1;
    }
}
