//! feature: An `i32` holding more than a card (50 * 50 = 2500), divided back onto one
//! section: Integer types
//! disposition: never
//! note: A cell holds -999..=999, so an `i32` past a card cannot be held, even for a moment.
//! input: 50 50
//! differs: The product 2500 is past a card: with overflow checks on CARDIAC panics, and with them off it keeps the low three digits (500), so it prints 50 where Rust prints 250.
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: i32 = inp() as i32;
    let b: i32 = inp() as i32;
    let p = a * b;
    out((p / 10) as i16);
}
