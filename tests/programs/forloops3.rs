#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

pub fn main() {
    let mut sum: usize = 0;
    for j in 0..=3usize {
        sum += j;
    }
    out(sum as i16);
    let mut xs = [0u8; 4];
    for j in 0..4 {
        xs[j] = j as u8 * 3;
    }
    for b in (0..=3u8).rev() {
        out(xs[b as usize] as i16);
    }
}
