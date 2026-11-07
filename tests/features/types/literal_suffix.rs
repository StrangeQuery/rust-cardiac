//! feature: Integer literals with type suffixes (`5i16`, `3u8`, `2usize`)
//! section: Integer literals
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
pub fn main() {
    let a = inp();
    out(a + 5i16);
    out((U[a as usize] + 3u8) as i16);
    out((a as usize + 2usize) as i16);
}
