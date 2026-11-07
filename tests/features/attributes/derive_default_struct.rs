//! feature: `#[derive(Default)]` on a struct: `Default::default()` and `..Default::default()`
//! section: Derive
//! disposition: easy
//! note: At opt-level 1 rustc calls `<i16 as Default>::default`, which is just 0.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Default)]
struct P { x: i16, y: i16, on: bool }
pub fn main() {
    let a: P = Default::default();
    let b = P { x: inp(), ..Default::default() };
    out(a.x); out(a.y); out(a.on as i16);
    out(b.x); out(b.y);
}
