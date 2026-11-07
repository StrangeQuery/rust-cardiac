//! feature: `println!`, with `extern crate std`
//! section: Standard library macros
//! disposition: never
//! note: Needs `std` and string formatting; `out()` prints a value.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
extern crate std;
pub fn main() {
    let a = inp();
    std::eprintln!("{}", a);
    out(a);
}
