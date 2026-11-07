//! feature: `a.abs_diff(b)` on `u8`, both orders of every end, through casts of negative cards
//! section: Sign and absolute value
//! input: 0 255 255 0 255 255 0 0 -1 0 0 -1 1 -1 -128 127 127 -128 200 7 7 200 999 999
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let x = inp();
        if x == 999 {
            break;
        }
        let a = x as u8;
        let b = inp() as u8;
        out(a.abs_diff(b) as i16);
    }
}
