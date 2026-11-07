//! feature: `#[repr(u8)]` on an enum with data, discriminants given
//! section: Type layout (repr)
//! input: 1 5 2 6 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(u8)]
#[derive(Clone, Copy)]
enum E { A(i16) = 1, B(i16) = 2 }
#[inline(never)]
fn val(e: E) -> i16 { match e { E::A(v) => v, E::B(v) => -v } }
pub fn main() {
    loop {
        let k = inp();
        if k == 0 { break; }
        let v = inp();
        out(val(if k == 1 { E::A(v) } else { E::B(v) }));
    }
}
