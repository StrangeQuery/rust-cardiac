//! feature: `let Some(x) = ... else { ... };`
//! section: If and if let expressions
//! input: 4 -4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pos(x: i16) -> Option<i16> { if x > 0 { Some(x) } else { None } }
#[inline(never)]
fn f(a: i16) -> i16 {
    let Some(v) = pos(a) else { return -1 };
    v * 3
}
pub fn main() {
    out(f(inp()));
    out(f(inp()));
}
