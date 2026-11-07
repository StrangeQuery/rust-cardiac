//! feature: `#[no_mangle]` written without `unsafe(...)` (edition 2021)
//! section: Linkage attributes
//! note: In edition 2024 this form is an error; the row shows which edition the build uses.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[no_mangle]
#[inline(never)]
pub fn helper(a: i16) -> i16 { a + 3 }
pub fn main() {
    out(helper(inp()));
}
