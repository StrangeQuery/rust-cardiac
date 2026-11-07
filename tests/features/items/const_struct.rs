//! feature: A `const` struct
//! section: Constant items
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Cfg { lo: i16, hi: i16 }
const C: Cfg = Cfg { lo: -10, hi: 10 };
pub fn main() {
    let v = inp();
    out(if v < C.lo { C.lo } else if v > C.hi { C.hi } else { v });
}
