#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// A quotient by a variable that shares the divisor's cell (`_3 = Div(const 200_u8, copy _3)`): counted in a scratch
/// cell, where counting in the divisor's own cell changed what each step subtracted, and printed 20.
#[inline(never)]
fn by(k: u8) {
    let x = k * 2;
    let v: u8 = 200 / x;
    out(v);
}

pub fn main() { by(3); by(50); by(1); }
