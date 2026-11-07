//! feature: `#[derive(Ord)]` on a struct: `.cmp()` matched on `Ordering`
//! section: Derive
//! disposition: easy
//! input: 1 2 1 3 2 0 1 9 4 4 4 4 0 0 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::cmp::Ordering;
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct P { x: i16, y: i16 }
pub fn main() {
    loop {
        let a = P { x: inp(), y: inp() };
        let b = P { x: inp(), y: inp() };
        if a.x == 0 && b.x == 0 { break; }
        out(match a.cmp(&b) { Ordering::Less => -1, Ordering::Equal => 0, Ordering::Greater => 1 });
    }
}
