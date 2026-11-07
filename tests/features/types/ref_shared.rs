//! feature: A shared reference `&i16` passed to a function
//! section: Pointer types
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn twice(v: &i16) -> i16 {
    *v + *v
}
pub fn main() {
    let a = inp();
    out(twice(&a));
}
