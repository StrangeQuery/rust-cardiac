//! feature: `#[expect(...)]`
//! section: Diagnostic attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[expect(dead_code)]
fn unused() -> i16 { 1 }
pub fn main() {
    #[expect(unused_variables)]
    let b = 3;
    out(inp());
}
