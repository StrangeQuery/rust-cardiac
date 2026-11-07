//! feature: A string literal's bytes, indexed by an input
//! section: Textual types
//! disposition: demand
//! note: A `&str` passed to a function is refused (RCE0019: the length of the slice it is passed isn't known here), though its `.as_bytes()` can be passed.
//! input: 0 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let s: &str = "hello";
    let b = s.as_bytes();
    let i = inp() as usize;
    let j = inp() as usize;
    out(b[i] as i16);
    out(b[j] as i16);
}
