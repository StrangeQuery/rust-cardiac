//! feature: a macro taking a `$b:block` and repeating it
//! section: Macros by example
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! thrice {
    ($b:block) => {
        $b
        $b
        $b
    };
}
pub fn main() {
    thrice!({
        out(inp() * 2);
    });
}
