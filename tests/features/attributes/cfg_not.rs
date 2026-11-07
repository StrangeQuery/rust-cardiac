//! feature: `#[cfg(not(...))]` on a statement and a block
//! section: Conditional compilation
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = inp();
    #[cfg(not(any()))]
    { a += 10; }
    #[cfg(any())]
    { a += 500; }
    out(a);
}
