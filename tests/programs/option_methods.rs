#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Option's methods, which rustc calls at opt-level 1: unwrap_or, is_some, is_none, and unwrap, which panics
/// on None.
#[inline(never)]
fn pos(v: i16) -> Option<i16> {
    if v >= 0 {
        Some(v + 1)
    } else {
        None
    }
}

pub fn main() {
    let mut v = -2;
    while v < 2 {
        let p = pos(v);
        out(p.unwrap_or(-5));
        if p.is_some() {
            out(1);
        }
        if p.is_none() {
            out(2);
        }
        v += 1;
    }
    out(pos(-v).unwrap());
}
