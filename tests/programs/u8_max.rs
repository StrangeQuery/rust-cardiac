#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// rustc prints 255_u8 by name, `u8::MAX`, and that was refused: "unsupported place const u8::MAX". It is
/// read as 255, a constant like any other: in an array,
#[inline(never)]
fn array(i: usize) {
    let us: [u8; 3] = [0, 9, 255];
    out(us[i] as i16);
}

/// in a comparison, and in a sum that fits.
#[inline(never)]
fn operand(x: u8) {
    out((x == 255) as i16);
    out((x < u8::MAX) as i16);
    if x == 0 {
        out((x + 255) as i16);
    }
}

pub fn main() {
    array(1);
    array(2);
    operand(255);
    operand(0);
}
