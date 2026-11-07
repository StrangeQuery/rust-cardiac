//! feature: `match` destructuring a struct, `P { x: 0, y }`
//! section: Match expressions
//! input: 0 5 3 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
pub fn main() {
    for _ in 0..2 {
        let p = P { x: inp(), y: inp() };
        out(match p { P { x: 0, y } => y, P { x, y } => x + y * 10 });
    }
}
