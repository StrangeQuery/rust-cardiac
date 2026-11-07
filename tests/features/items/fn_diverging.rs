//! feature: A diverging function (`-> !`)
//! section: Functions
//! disposition: easy
//! note: A call that never returns needs nothing after it; the second card is negative, so the program then panics.
//! input: 3 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn fail() -> ! { panic!() }
fn check(x: i16) -> i16 { if x < 0 { fail() } else { x } }
pub fn main() {
    out(check(inp()));
    out(check(inp()));
}
