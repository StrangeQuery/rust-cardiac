//! feature: a `fn` declared inside a function
//! section: Item declarations
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    fn twice(x: i16) -> i16 {
        x + x
    }
    let a = inp();
    out(twice(a));
    out(twice(inp()));
}
