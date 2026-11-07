//! feature: `#[derive(Ord)]` on a C-like enum: `.cmp()` and `.max()`
//! section: Derive
//! disposition: easy
//! input: 0 2 2 1 1 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::cmp::Ordering;
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Lvl { Low, Mid, High }
fn lvl(v: i16) -> Lvl { if v == 0 { Lvl::Low } else if v == 1 { Lvl::Mid } else { Lvl::High } }
pub fn main() {
    loop {
        let a = inp();
        if a < 0 { break; }
        let b = inp();
        let (x, y) = (lvl(a), lvl(b));
        out(match x.cmp(&y) { Ordering::Less => -1, Ordering::Equal => 0, Ordering::Greater => 1 });
        out(x.max(y) as i16);
    }
}
