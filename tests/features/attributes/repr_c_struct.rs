//! feature: `#[repr(C)]` and `#[repr(transparent)]` on structs
//! section: Type layout (repr)
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(C)]
#[derive(Clone, Copy)]
struct P { x: i16, y: i16 }
#[repr(transparent)]
#[derive(Clone, Copy)]
struct Meters(i16);
#[inline(never)]
fn len(p: P, m: Meters) -> i16 { p.x + p.y + m.0 }
pub fn main() {
    let p = P { x: inp(), y: inp() };
    out(len(p, Meters(10)));
}
