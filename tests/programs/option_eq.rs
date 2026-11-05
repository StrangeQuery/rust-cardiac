#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `==` and `!=` of two Options, which rustc calls at opt-level 1 and s (`<Option<i16> as PartialEq>::eq`):
/// the same variant, and for Some the same payload. (`== None` passes a promoted constant: niche_eq.rs, at 2.)
#[inline(never)]
fn lower(v: i16) -> Option<i16> {
    if v < 1 {
        Some(v - 1)
    } else {
        None
    }
}

pub fn main() {
    let mut v = -3;
    while v < 4 {
        out(if lower(v) == lower(v + 2) { 1 } else { 0 });
        v += 1;
    }
}
