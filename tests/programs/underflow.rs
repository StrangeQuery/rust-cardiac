#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Counts down past zero. Overflow checks make the subtraction at 0 panic instead of wrapping to 255.
pub fn main() {
    let mut n: u8 = 2;
    loop {
        out(n);
        n -= 1;
    }
}
