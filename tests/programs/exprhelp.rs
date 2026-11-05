#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Expression form: small helpers that return a value. Hours on a 24-hour clock, and how far each is from noon.
fn hour(h: i16) -> i16 { if h < 0 { h + 24 } else if h >= 24 { h - 24 } else { h } }
fn abs(v: i16) -> i16 { if v < 0 { -v } else { v } }

pub fn main() {
    let mut h = 7;
    let mut k = -12;
    loop {
        h = hour(h + 17);
        out(abs(h - 12));
        k += 1;
        if k >= 0 { break; }
    }
}
