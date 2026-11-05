#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

#[inline(never)]
pub fn add(a: u8, b: u8) -> u8 { a + b }

#[inline(never)]
pub fn triple(x: u8) -> u8 { add(add(x, x), x) }

/// A function that calls another function: the return instruction in cell 99 is saved before it is overwritten.
pub fn main() {
    let mut i: u8 = 1;
    while i < 6 {
        out(triple(i));
        i = add(i, 1);
    }
}
