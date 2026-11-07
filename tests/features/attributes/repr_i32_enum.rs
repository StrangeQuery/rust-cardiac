//! feature: `#[repr(i32)]` on a C-like enum whose numbers fit on a card
//! section: Type layout (repr)
//! input: 0 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(i32)]
#[derive(Clone, Copy)]
enum C { A = -9, B = 900 }
pub fn main() {
    loop {
        let k = inp();
        if k < 0 { break; }
        let c = if k == 0 { C::A } else { C::B };
        out(c as i16);
    }
}
