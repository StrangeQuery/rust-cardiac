#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// The `?` operator on a Result, within a function: what rustc calls `Try::branch` and
/// `FromResidual::from_residual` at opt-level 1.
#[inline(never)]
fn parse(v: i16) -> Result<i16, i16> {
    if v < 0 {
        Err(v)
    } else {
        Ok(v)
    }
}

#[inline(never)]
fn twice(v: i16) -> Result<i16, i16> {
    let a = parse(v)?;
    Ok(a + a)
}

pub fn main() {
    let mut v = -1;
    while v < 2 {
        match twice(v) {
            Ok(x) => out(x),
            Err(e) => out(e),
        }
        v += 1;
    }
}
