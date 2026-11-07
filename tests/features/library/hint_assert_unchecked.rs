//! feature: `core::hint::assert_unchecked`
//! section: Hints (core::hint)
//! input: 2 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [10, 20, 30, 40];
    let i = inp() as usize;
    unsafe { core::hint::assert_unchecked(i < 4) };
    out(t[i]);
    out(inp());
}
