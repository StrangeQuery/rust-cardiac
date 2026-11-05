#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// An i16 is a card: past 999 it panics on CARDIAC, where native Rust would carry on to 32767. Expected on
/// CARDIAC: 2, 5, 14, 41, 122, 365, -999 (the panic). Not native-diffed, since native differs here by design.
pub fn main() {
    let mut k: i16 = 1;
    loop { k = k * 3 - 1; out(k); }
}
