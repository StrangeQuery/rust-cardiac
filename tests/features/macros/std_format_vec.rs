//! feature: `format!` and `vec!`, with `extern crate std`
//! section: Standard library macros
//! disposition: never
//! note: Both need the heap.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
extern crate std;
pub fn main() {
    let a = inp();
    let s = std::format!("{}", a);
    let v = std::vec![a, a];
    out(s.len() as i16 + v.len() as i16);
}
