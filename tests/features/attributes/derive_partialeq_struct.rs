//! feature: `#[derive(PartialEq)]` on a struct: `==` and `!=`
//! section: Derive
//! input: 1 2 1 2 1 2 1 3 5 2 1 2 0 0 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(PartialEq)]
struct P { x: i16, y: i16 }
pub fn main() {
    loop {
        let a = P { x: inp(), y: inp() };
        let b = P { x: inp(), y: inp() };
        if a.x == 0 && b.x == 0 { break; }
        out(if a == b { 1 } else { 0 });
        out(if a != b { 1 } else { 0 });
    }
}
