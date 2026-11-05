#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Walks one step past the end of the array. The bounds check panics instead of reading the next cell.
pub fn main() {
    let a: [u8; 3] = [7, 8, 9];
    let mut i: usize = 0;
    loop {
        out(a[i]);
        i += 1;
    }
}
