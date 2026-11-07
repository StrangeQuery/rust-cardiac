//! feature: `_ = f()`: calling and dropping the result
//! section: Underscore expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn noisy(a: i16) -> i16 { out(a); a + 1 }
pub fn main() {
    let a = inp();
    _ = noisy(a);
    let _ = noisy(a + 1);
}
