//! feature: An array of function pointers indexed by an input
//! section: Function pointer types
//! disposition: demand
//! input: 1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn double(x: i16) -> i16 { x * 2 }
#[inline(never)]
fn neg(x: i16) -> i16 { -x }
pub fn main() {
    let t: [fn(i16) -> i16; 2] = [double, neg];
    let i = inp() as usize;
    out(t[i](inp()));
}
