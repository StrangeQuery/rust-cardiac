#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn index(k: usize) -> usize {
    k
}

/// `x & m` on a u8 and a usize, m one below a power of two: x % (m + 1).
pub fn main() {
    let mut b: u8 = 250;
    while b > 200 {
        out((b & 15) as i16);
        b -= 7;
    }
    let mut i = index(0);
    while i < 40 {
        out((index(i) & 31) as i16);
        i += 7;
    }
}
