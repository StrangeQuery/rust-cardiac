//! feature: A C-like enum with given discriminants, cast with `as`
//! section: Enumerations
//! input: 0 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Level { Low = -5, Mid = 10, High = 900 }
fn pick(v: i16) -> Level { match v { 0 => Level::Low, 1 => Level::Mid, _ => Level::High } }
pub fn main() {
    for _ in 0..3 { out(pick(inp()) as i16); }
}
