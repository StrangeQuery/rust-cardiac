//! feature: `u8::abs_diff` against constants, and with a u8 array element
//! section: Sign and absolute value
//! input: 0 1 128 254 255 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const T: [u8; 3] = [0, 128, 255];
pub fn main() {
    let mut i = 0;
    loop {
        let x = inp();
        if x < 0 {
            break;
        }
        let a = x as u8;
        out(a.abs_diff(255) as i16);
        out(0u8.abs_diff(a) as i16);
        out(T[i % 3].abs_diff(a) as i16);
        i += 1;
    }
}
