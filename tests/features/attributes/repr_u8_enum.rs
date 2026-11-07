//! feature: `#[repr(u8)]` on a C-like enum, `as u8` and `as i16`
//! section: Type layout (repr)
//! input: 0 1 2 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(u8)]
#[derive(Clone, Copy)]
enum C { A = 7, B = 200, Z = 255 }
pub fn main() {
    loop {
        let k = inp();
        if k < 0 { break; }
        let c = if k == 0 { C::A } else if k == 1 { C::B } else { C::Z };
        out(c as u8 as i16);
    }
}
