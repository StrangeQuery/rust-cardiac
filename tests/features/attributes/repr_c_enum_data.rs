//! feature: `#[repr(C)]` on an enum with data
//! section: Type layout (repr)
//! input: 1 5 2 6 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(C)]
#[derive(Clone, Copy)]
enum F { X(i16), Y(i16) }
#[inline(never)]
fn val(f: F) -> i16 { match f { F::X(v) => v, F::Y(v) => -v } }
pub fn main() {
    loop {
        let k = inp();
        if k == 0 { break; }
        let v = inp();
        out(val(if k == 1 { F::X(v) } else { F::Y(v) }));
    }
}
