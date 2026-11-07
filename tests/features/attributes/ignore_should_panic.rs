//! feature: `#[ignore]` and `#[should_panic]` beside `#[test]`
//! section: Testing attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[test]
#[ignore]
fn slow() {}
#[test]
#[should_panic]
fn fails() { panic!(); }
pub fn main() {
    out(inp());
}
