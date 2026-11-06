#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// What rustc inlines at opt-level 2 for checked_neg and checked_div tests for i16::MIN, which no card holds: what
/// is left is the test of the rest (a divisor of 0).
#[inline(never)]
fn show(a: i16, b: i16) {
    match a.checked_neg() {
        Some(v) => out(v),
        None => out(-1),
    }
    match a.checked_div(b) {
        Some(v) => out(v),
        None => out(-1),
    }
}

pub fn main() {
    show(7, 3);
    show(-999, -1);
    show(999, 0);
}
