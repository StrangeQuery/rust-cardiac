//! feature: `while let Some(x) = ...`
//! section: Loop expressions
//! input: 4 9 2 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn next() -> Option<i16> {
    let a = inp();
    if a < 0 { None } else { Some(a) }
}
pub fn main() {
    while let Some(x) = next() {
        out(x * 2);
    }
}
