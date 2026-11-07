//! feature: A nested tuple returned from a function
//! section: Tuple types
//! disposition: demand
//! input: 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn make(a: i16, b: i16) -> ((i16, i16), bool) {
    ((a + b, a - b), a < b)
}
pub fn main() {
    let a = inp();
    let b = inp();
    let ((s, d), lt) = make(a, b);
    out(s);
    out(d);
    out(lt as i16);
}
