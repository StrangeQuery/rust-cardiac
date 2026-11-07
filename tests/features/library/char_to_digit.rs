//! feature: `char::to_digit(10)`
//! section: bool and char
//! disposition: demand
//! note: The compiler has no `char` type yet, and `to_digit` returns an `Option<u32>`.
//! input: 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const CH: [u8; 4] = [b'0', b'7', b'a', b' '];
pub fn main() {
    for _ in 0..2 {
        let b = CH[inp() as usize];
        let c = b as char;
        out(match c.to_digit(10) { Some(d) => d as i16, None => -1 });
    }
}
