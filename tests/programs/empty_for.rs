#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A `for` over an empty range in code never reached: rustc leaves the loop out of the MIR but keeps its
/// `Range` local, which no line names and so needs no cell. Beside it, a loop that runs.
pub fn main() {
    let xs: [i16; 2] = [3, -2];
    if false {
        for i in 0..0 {
            out(i);
        }
    }
    let mut k: usize = 0;
    while k < 2 {
        for i in 0..xs[k] {
            out(i);
        }
        k += 1;
    }
}
