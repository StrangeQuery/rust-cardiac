//! feature: `==`, `!=`, `<` on `u8`
//! section: Comparison operators
//! input: 0 7 7 7 7 0 6 7 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = U[i as usize];
        let b = U[inp() as usize];
        out((a == b) as i16);
        out((a != b) as i16);
        out((a < b) as i16);
    }
}
