//! feature: A literal in a field (`P { x: 0, y }`) in a `match`
//! section: Struct patterns
//! input: 0 4 2 5 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
#[inline(never)]
fn make(x: i16, y: i16) -> P { P { x, y } }
pub fn main() {
    loop {
        let x = inp();
        let y = inp();
        if x == 0 && y == 0 { break; }
        let r = match make(x, y) {
            P { x: 0, y } => y,
            P { y: 0, x } => -x,
            P { .. } => 100,
        };
        out(r);
    }
}
