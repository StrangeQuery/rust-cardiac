//! feature: Unit enum variants (`Colour::Red =>`)
//! section: Path patterns
//! input: 0 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Colour { Red, Green, Blue }
#[inline(never)]
fn pick(v: i16) -> Colour { match v { 0 => Colour::Red, 1 => Colour::Green, _ => Colour::Blue } }
pub fn main() {
    for _ in 0..3 {
        let c = pick(inp());
        out(match c { Colour::Red => 10, Colour::Green => 20, Colour::Blue => 30 });
    }
}
