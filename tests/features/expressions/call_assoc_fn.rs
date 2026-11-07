//! feature: An associated function, `P::new(a, b)`
//! section: Call expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
impl P {
    #[inline(never)]
    fn new(x: i16, y: i16) -> P { P { x, y } }
}
pub fn main() {
    let p = P::new(inp(), inp());
    out(p.x - p.y);
}
