//! feature: An inline `const { ... }` block
//! section: Block expressions
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a + const { 6 * 7 });
}
