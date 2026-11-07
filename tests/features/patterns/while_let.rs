//! feature: `while let Some(x) = ...`
//! section: Destructuring
//! input: 4 2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn next() -> Option<i16> { let v = inp(); if v != 0 { Some(v) } else { None } }
pub fn main() {
    let mut total = 0;
    while let Some(x) = next() {
        total += x;
        out(total);
    }
}
