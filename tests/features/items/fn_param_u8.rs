//! feature: A function taking and returning `u8`, not inlined
//! section: Functions
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const T: [u8; 2] = [200, 50];
#[inline(never)]
fn f(a: u8, b: u8) -> u8 { a - b }
pub fn main() {
    let a = T[inp() as usize];
    let b = T[inp() as usize];
    out(f(a, b) as i16);
}
