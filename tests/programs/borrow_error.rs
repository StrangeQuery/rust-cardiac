#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Two live mutable borrows of one cell. rustc refuses before CARDIAC ever sees it.
pub fn main() {
    let mut cell: u8 = 7;
    let a = &mut cell;
    let b = &mut cell;
    *a += 1;
    *b += 1;
    out(cell);
}
