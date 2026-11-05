#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Fibonacci in u8 while it stays under 200.
pub fn main() {
    let mut a: u8 = 0;
    let mut b: u8 = 1;
    while a < 200 {
        out(a);
        let next = a + b;
        a = b;
        b = next;
    }
}
