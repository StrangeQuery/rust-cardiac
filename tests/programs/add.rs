#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

#[inline(never)]
pub fn add(a: u8, b: u8) -> u8 {
    a + b
}

/// Calls add() from a loop: prints 10, 30, 60, 100, 150, 210, then 210 + 70 does not fit in a u8 and panics.
pub fn main() {
    let mut total: u8 = 0;
    let mut step: u8 = 10;
    loop {
        total = add(total, step);
        out(total);
        step = add(step, 10);
    }
}
