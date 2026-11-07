//! feature: a `#[test]` function in the program
//! section: Testing attributes
//! note: Outside `cargo test` a `#[test]` function isn't compiled at all, so it costs nothing.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[test]
fn checks() {
    assert!(1 + 1 == 2);
}
pub fn main() {
    out(inp());
}
