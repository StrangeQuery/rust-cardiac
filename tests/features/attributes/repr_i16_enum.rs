//! feature: `#[repr(i16)]` on a C-like enum with numbers given, `as i16`
//! section: Type layout (repr)
//! input: 0 1 2 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(i16)]
#[derive(Clone, Copy)]
enum C { Low = -500, Mid = 3, High = 999 }
pub fn main() {
    loop {
        let k = inp();
        if k < 0 { break; }
        let c = if k == 0 { C::Low } else if k == 1 { C::Mid } else { C::High };
        out(c as i16);
    }
}
