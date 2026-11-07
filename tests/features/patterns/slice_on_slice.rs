//! feature: Slice pattern on a slice (`&[i16]`: `[a, ..]`, `[]`)
//! section: Slice patterns
//! disposition: demand
//! input: 4 5 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn first(s: &[i16]) -> i16 {
    match s {
        [] => 0,
        [a, ..] => *a,
    }
}
pub fn main() {
    let t = [inp(), inp(), inp()];
    out(first(&t));
    out(first(&t[1..]));
    out(first(&t[3..]));
}
