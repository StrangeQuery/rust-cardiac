//! feature: A `const` past a card (`const BIG: i16 = 1500`)
//! section: Constant items
//! note: Only `BIG / 10` (150) is used, and rustc works it out before the compiler sees it; `BIG` used at run time, as in `BIG / inp()`, is refused on purpose (RCE0009).
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const BIG: i16 = 1500;
pub fn main() {
    out(BIG / 10 + inp());
}
