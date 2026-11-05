#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const TOP: u8 = 255;

struct Pair {
    a: u8,
    b: i16,
}

#[inline(never)]
fn arg(a: u8) {
    out(a as i16);
}

#[inline(never)]
fn field(p: Pair) {
    out(p.a as i16 - p.b);
}

#[inline(never)]
fn make(b: i16) {
    field(Pair { a: 255, b });
}

/// `u8::MAX`, as rustc prints 255_u8, is 255 as an argument, a field and a const item. It was refused.
pub fn main() {
    arg(255);
    arg(TOP);
    make(200);
}
