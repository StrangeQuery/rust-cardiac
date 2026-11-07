//! feature: `**r` through a reference to a reference
//! section: Dereference operator
//! disposition: demand
//! note: A reference to a reference is an address held in a cell, which the compiler doesn't follow twice yet.
//! input: 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn get(r: &&i16) -> i16 { **r + 1 }
pub fn main() {
    let a = inp();
    let r = &a;
    out(get(&r));
}
