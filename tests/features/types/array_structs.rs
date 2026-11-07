//! feature: An array of structs
//! section: Array types
//! input: 1 2 3 4 5 6 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P {
    x: i16,
    y: i16,
}
#[inline(never)]
fn make(x: i16, y: i16) -> P {
    P { x, y }
}
pub fn main() {
    let mut t = [P { x: 0, y: 0 }; 3];
    let mut k = 0;
    while k < 3 {
        t[k] = make(inp(), inp());
        k += 1;
    }
    let i = inp() as usize;
    out(t[i].x + t[i].y);
}
