//! feature: `i16::from_str_radix(s, 10)`
//! section: Conversions
//! disposition: never
//! note: A program's input is numbers on cards, so there is no text to parse; the strings to parse could only be literals, a cell a byte.
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const S: [&str; 2] = ["12", "-7"];
pub fn main() {
    loop {
        let i = inp();
        match i16::from_str_radix(S[i as usize], 10) {
            Ok(v) => out(v),
            Err(_) => out(-1),
        }
        if i == 1 {
            break;
        }
    }
}
