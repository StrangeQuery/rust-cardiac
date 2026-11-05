#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// i16's unchecked_neg, which a card's i16 always passes; and u8's unchecked_add past 255, which fails: with debug assertions on, core's check of its precondition panics.
#[inline(never)]
fn add(a: u8, b: u8) -> u8 {
    unsafe { a.unchecked_add(b) }
}

#[inline(never)]
fn neg(a: i16) -> i16 {
    unsafe { a.unchecked_neg() }
}

pub fn main() {
    out(neg(-7));
    out(add(200, 55) as i16);
    out(add(200, 56) as i16);
}
