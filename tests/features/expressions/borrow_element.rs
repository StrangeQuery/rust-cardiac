//! feature: `&mut t[i]` to an array element, passed to a function
//! section: Borrow operators
//! input: 1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn bump(v: &mut i16, by: i16) { *v += by; }
pub fn main() {
    let mut t = [10, 20, 30];
    let i = inp() as usize;
    let b = inp();
    bump(&mut t[i], b);
    out(t[0]);
    out(t[1]);
    out(t[2]);
}
