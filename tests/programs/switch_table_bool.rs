#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

/// A bool and a u8 below 4 as one value, 0 to 7.
#[inline(never)]
fn pair(b: bool, k: u8) -> i16 {
    unsafe { assume(k < 4) };
    match b as u8 + b as u8 + b as u8 + b as u8 + k {
        0 => 20,
        1 => 21,
        2 => 22,
        3 => 23,
        4 => 24,
        5 => 25,
        6 => 26,
        _ => 27,
    }
}

/// A dense match on a value made of a bool and a u8.
pub fn main() {
    let mut k: u8 = 0;
    while k < 4 {
        out(pair(false, k));
        out(pair(true, k));
        k += 1;
    }
}
