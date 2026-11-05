#![no_std]
#[inline(never)]
pub fn inp() -> i16 {
    0
}
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn add_times(c: i16, n: i16) -> i16 {
    let mut sum = 0;
    let mut k = 0;
    while k < n {
        sum += c;
        k += 1;
    }
    sum
}

/// One add_times(c, n), c and n from the deck: test_compiler.sh checks the product, and the instructions the
/// run takes for each n (the routine's 24, and one an addition: n = 37 is 3 for the tens and 7 for the units).
pub fn main() {
    let c = inp();
    let n = inp();
    out(add_times(c, n));
}
