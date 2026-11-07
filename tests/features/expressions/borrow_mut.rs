//! feature: `&mut x` passed to a function that changes it
//! section: Borrow operators
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn bump(v: &mut i16) { *v += 10; }
pub fn main() {
    let mut a = inp();
    bump(&mut a);
    bump(&mut a);
    out(a);
}
