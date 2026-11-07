//! feature: A mutable reference `&mut i16` passed to a function
//! section: Pointer types
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn bump(v: &mut i16) {
    *v += 1;
}
pub fn main() {
    let mut a = inp();
    bump(&mut a);
    bump(&mut a);
    out(a);
}
