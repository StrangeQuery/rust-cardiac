#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Parameters passed straight on: to the multiply routine, and to another function kept as a call.
#[inline(never)]
fn multiply(x: i16, y: i16) -> i16 { x * y }

#[inline(never)]
fn area(w: i16, h: i16) -> i16 { multiply(w, h) }

/// c and d are passed on after a multiply, which would overwrite the routine's cells: they keep their own.
#[inline(never)]
fn both(a: i16, b: i16, c: i16, d: i16) -> i16 { a * b - multiply(c, d) }

pub fn main() {
    let mut p: i16 = -7;
    while p < 9 {
        out(area(p, 5 - p));
        out(both(p, -3, p, 4));
        p += 4;
    }
}
