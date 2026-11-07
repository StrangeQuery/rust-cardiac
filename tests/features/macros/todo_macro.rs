//! feature: `todo!()`
//! section: Panics
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn later(_x: i16) -> i16 {
    todo!()
}
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let v = if a > 0 { a } else { later(a) };
        out(v);
    }
}
