//! feature: A type alias for a scalar (`type Card = i16`)
//! section: Type aliases
//! input: 42
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
type Card = i16;
fn bump(c: Card) -> Card { c + 1 }
pub fn main() {
    let c: Card = inp();
    out(bump(c));
}
