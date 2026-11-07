//! feature: Calling a function that isn't inlined
//! section: Call expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn f(a: i16, b: i16) -> i16 { a * 2 + b }
pub fn main() {
    let a = inp();
    let b = inp();
    out(f(a, b));
    out(f(b, a));
}
