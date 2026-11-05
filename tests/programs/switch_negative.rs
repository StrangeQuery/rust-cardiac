#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A comparison with a negative constant is a switchInt whose arm MIR prints as its bits: `x == -1` of an i16
/// is `[65535: ..]`. Each arm is read as the switch's type: here in an `if`, with == and != and on a checked op's
/// result.
pub fn main() {
    let xs: [i16; 4] = [2, -1, -99, 7];
    let mut k: usize = 0;
    while k < 4 {
        let x = xs[k];
        out(if 1 - x == -1 { 1 } else { 0 });
        out(if x != -99 { 3 } else { 4 });
        k += 1;
    }
}
