//! feature: A trait implemented for an enum
//! section: Implementations
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Coin { Penny, Dime }
trait Value { fn cents(&self) -> i16; }
impl Value for Coin { fn cents(&self) -> i16 { match self { Coin::Penny => 1, Coin::Dime => 10 } } }
fn pick(v: i16) -> Coin { if v == 0 { Coin::Penny } else { Coin::Dime } }
pub fn main() {
    let a = pick(inp());
    let b = pick(inp());
    out(a.cents() + b.cents());
}
