//! feature: `Option` of a C-like enum, whose `None` is a spare variant number
//! section: Type layout
//! disposition: easy
//! note: The refusal is a constant `Some` of a C-like enum, which is just two numbers.
//! input: 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Colour {
    Red = 1,
    Blue = 2,
}
#[inline(never)]
fn make(k: i16) -> Option<Colour> {
    if k > 1 { Some(Colour::Blue) } else if k > 0 { Some(Colour::Red) } else { None }
}
pub fn main() {
    let mut n = 0;
    while n < 2 {
        match make(inp()) {
            Some(c) => out(c as i16),
            None => out(-1),
        }
        n += 1;
    }
}
