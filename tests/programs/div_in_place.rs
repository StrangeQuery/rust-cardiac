#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A quotient by a constant that shares its dividend's cell (`_5 = UDiv(copy _5, const 3_i16)`, x and v one cell):
/// the loop counts it in a scratch cell, where zeroing its own cell first lost the dividend and printed 0.
/// Then a remainder in its dividend's cell, which the loop works out in the accumulator.
#[inline(never)]
fn third(k: i16) {
    let x = k * 2;
    unsafe { core::hint::assert_unchecked(x >= 1 && x <= 12) };
    let v: i16 = x / 3;
    out(v);
}

#[inline(never)]
fn seventh(k: i16) {
    let x = k * 2;
    unsafe { core::hint::assert_unchecked(x >= 1 && x <= 12) };
    let v: i16 = x % 7;
    out(v);
}

pub fn main() { third(3); third(6); third(1); seventh(6); seventh(2); }
