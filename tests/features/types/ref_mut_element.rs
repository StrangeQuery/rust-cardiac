//! feature: A `&mut` to an array element chosen by an input
//! section: Pointer types
//! input: 1 2 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn bump(v: &mut i16) {
    *v += 10;
}
pub fn main() {
    let mut t = [inp(), inp(), inp()];
    let i = inp() as usize;
    bump(&mut t[i]);
    out(t[0]);
    out(t[1]);
    out(t[2]);
}
