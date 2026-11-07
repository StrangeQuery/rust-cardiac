//! feature: Integer arithmetic with no type given (`i32`, Rust's default)
//! section: Arithmetic operators
//! input: 7 3 -20 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut x = 5;
    let mut i = 0;
    while i < 2 {
        let a = inp();
        let b = inp();
        x = x * 2 + (a - b) as i32;
        out(x as i16);
        i += 1;
    }
}
