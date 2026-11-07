//! feature: Nested calls, `f(g(a), g(b))`
//! section: Call expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn g(a: i16) -> i16 { a + 1 }
#[inline(never)]
fn f(a: i16, b: i16) -> i16 { a - b }
pub fn main() {
    let a = inp();
    let b = inp();
    out(f(g(a), g(g(b))));
}
