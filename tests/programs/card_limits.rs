#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// No card holds i16::MIN or i16::MAX: a comparison with one is settled by the card, and so is a switch arm.
#[inline(never)]
fn show(a: i16) {
    out(if a < i16::MAX { 1 } else { 0 });
    out(if i16::MIN < a { 1 } else { 0 });
    out(if a == i16::MIN { 1 } else { 0 });
    out(match a {
        i16::MIN => 7,
        i16::MAX => 8,
        _ => 9,
    });
}

pub fn main() {
    show(7);
    show(-999);
    show(999);
}
