//! feature: A function with no result (`-> ()`) that prints
//! section: Functions
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn show_twice(x: i16) { out(x); out(x); }
pub fn main() {
    show_twice(inp());
    show_twice(1);
}
