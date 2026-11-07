//! feature: A mutable slice `&mut [i16]` written by a function
//! section: Slice types
//! disposition: demand
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn double(s: &mut [i16]) {
    let mut i = 0;
    while i < s.len() {
        s[i] = s[i] * 2;
        i += 1;
    }
}
pub fn main() {
    let mut a = [inp(), inp(), inp()];
    double(&mut a);
    out(a[0]);
    out(a[1]);
    out(a[2]);
}
