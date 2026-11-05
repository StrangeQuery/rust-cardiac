#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 4] = [10, 11, 12, 13];

/// A negative i16 as usize is a usize past a card, usize::MAX for -1, and in Rust that is no error: it is
/// held as the negative number it is, and compares above every usize a card holds, casts back to the i16 it was and
/// to u8 mod 256 (-300 is 212). -1 as usize printed -999 instead, the cast's own panic.
#[inline(never)]
fn uses(m: i16) {
    let a = (m - 1) as usize;
    out(a as i16);
    out((a as u8) as i16);
    out((a < 4) as i16);
}

/// Used as an index, it fails the bounds check, as in Rust.
#[inline(never)]
fn index(m: i16) {
    out(T[(m - 1) as usize]);
}

pub fn main() {
    uses(3);
    uses(0);
    uses(-299);
    index(4);
    index(0);
}
