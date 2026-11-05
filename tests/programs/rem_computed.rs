#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// An unsigned remainder and quotient by a variable through a &mut: the subtracting loop, stored through the
/// reference.
#[inline(never)]
fn rem(r: &mut u8, x: u8, y: u8) { *r = x % y; }

#[inline(never)]
fn div(r: &mut u8, x: u8, y: u8) { *r = x / y; }

pub fn main() {
    let mut v: u8 = 1;
    rem(&mut v, 250, 7);
    out(v);
    div(&mut v, 250, 7);
    out(v);
}
