#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Quotient and remainder by a runtime divisor, then a division by zero, which panics.
pub fn main() {
    let mut d: u8 = 4;
    loop {
        out(200 / d);
        out(200 % d);
        d -= 2;
    }
}
