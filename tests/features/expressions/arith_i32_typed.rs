//! feature: Arithmetic on a variable declared `i32`
//! section: Arithmetic operators
//! disposition: demand
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: i32 = inp() as i32;
    let b: i32 = inp() as i32;
    out((a * b + 1) as i16);
}
