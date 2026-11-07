//! feature: A reference to a reference `&&i16`
//! section: Pointer types
//! disposition: demand
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn get(v: &&i16) -> i16 {
    **v
}
pub fn main() {
    let a = inp();
    let r = &a;
    out(get(&r) + 1);
}
