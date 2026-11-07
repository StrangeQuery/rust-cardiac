//! feature: `a.div_euclid(b)` by a variable that is 0: Rust's panic
//! section: Euclidean division
//! input: -10 3 -10 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = inp();
        out(a.div_euclid(b));
    }
}
