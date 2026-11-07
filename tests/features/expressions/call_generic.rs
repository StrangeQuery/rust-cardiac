//! feature: A generic function, called with two types
//! section: Call expressions
//! disposition: demand
//! note: Generic functions are compiled only when rustc inlines them; each instance would need its own copy.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn first<T: Copy>(a: T, _b: T) -> T { a }
pub fn main() {
    let a = inp();
    out(first(a, 1));
    out(first(a as u8, 2u8) as i16);
}
