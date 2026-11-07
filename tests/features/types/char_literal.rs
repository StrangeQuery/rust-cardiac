//! feature: `char` literals compared with an input (`'0'..='9'`)
//! section: Textual types
//! disposition: easy
//! input: 55 65
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut k = 0;
    while k < 2 {
        let c = inp() as u8 as char;
        out((c >= '0' && c <= '9') as i16);
        out((c as i16) - ('0' as i16));
        k += 1;
    }
}
