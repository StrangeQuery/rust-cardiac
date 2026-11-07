//! feature: `char::from_digit`
//! section: bool and char
//! disposition: demand
//! note: The compiler has no `char` type yet, and `from_digit` takes a `u32`.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let d = inp();
    let c = char::from_digit(d as u32, 10).unwrap();
    out(c as u8 as i16);
}
