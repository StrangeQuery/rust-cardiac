//! feature: `include!` of a Rust expression
//! section: Other core macros
//! file: include_value.txt
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a + include!("include_value.txt"));
}
