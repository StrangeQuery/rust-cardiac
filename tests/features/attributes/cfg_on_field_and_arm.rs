//! feature: `#[cfg]` on a struct field, a match arm and an expression in a list
//! section: Conditional compilation
//! input: 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P {
    x: i16,
    #[cfg(any())]
    gone: i16,
    y: i16,
}
pub fn main() {
    let p = P { x: inp(), #[cfg(any())] gone: 0, y: inp() };
    let t = [p.x, #[cfg(any())] 99, p.y];
    match t[0] {
        #[cfg(any())]
        2 => out(500),
        2 => out(p.x + p.y),
        _ => out(t[1]),
    }
}
