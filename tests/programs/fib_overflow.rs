#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Fibonacci in u8 with no stopping condition: 144 + 233 doesn't fit, so the add panics.
pub fn main() {
    let mut a: u8 = 0;
    let mut b: u8 = 1;
    loop {
        out(a);
        let next = a + b;
        a = b;
        b = next;
    }
}
