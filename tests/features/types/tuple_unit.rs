//! feature: The unit type `()`: a function returning it, a tuple holding it
//! section: Tuple types
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn show(x: i16) -> () {
    out(x);
}
pub fn main() {
    let a = inp();
    let u: () = show(a);
    let t: ((), i16) = (u, a + 1);
    out(t.1);
}
