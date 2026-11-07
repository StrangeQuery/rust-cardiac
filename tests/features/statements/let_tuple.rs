//! feature: `let (a, b) = (e1, e2);`
//! section: Let statements
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let (a, b) = (inp(), inp());
    out(a - b);
    let (c, _) = (b, a);
    out(c);
}
