#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A signed quotient by a variable stored straight into an array element: the shared routine, every combination of
/// signs (the unsigned loop never ended on a negative operand).
#[inline(never)]
fn into(x: i16, y: i16) {
    let mut a: [i16; 2] = [12, 7];
    a[1] = x / y;
    out(a[0]);
    out(a[1]);
}

pub fn main() { into(-37, -7); into(37, -7); into(-6, -791); }
