#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// The `?` operator on an Option, within a function: what rustc calls `Try::branch` and
/// `FromResidual::from_residual` at opt-level 1.
#[inline(never)]
fn pos(v: i16) -> Option<i16> {
    if v >= 0 {
        Some(v)
    } else {
        None
    }
}

#[inline(never)]
fn up(v: i16) -> Option<i16> {
    let a = pos(v)?;
    Some(a + 100)
}

pub fn main() {
    let mut v = -1;
    while v < 2 {
        out(up(v).unwrap_or(-1));
        v += 1;
    }
}
