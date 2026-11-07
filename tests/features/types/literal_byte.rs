//! feature: Byte literals (`b'a'`, `b'\n'`), which are `u8`
//! section: Integer literals
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
pub fn main() {
    let a = U[inp() as usize];
    out((b'a' + a) as i16);
    out((b'\n' + a) as i16);
    out((a < b'0') as i16);
}
