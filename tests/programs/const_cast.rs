#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const WORDS: usize = 16;
const LIMIT: i16 = 250;
const FLAG: bool = true;

/// A constant's cast is its value as the type has it: a named usize as an i16, a named i16 as a u8 and as a
/// usize, a bool constant as a number.
#[inline(never)]
fn show(x: i16) {
    if (1..=WORDS as i16).contains(&x) {
        out(x);
    }
    let small: u8 = LIMIT as u8;
    if x < small as i16 {
        out(LIMIT as usize as i16 - x);
    }
    out(FLAG as i16 + x);
}

pub fn main() {
    show(3);
    show(16);
    show(17);
    show(249);
}
