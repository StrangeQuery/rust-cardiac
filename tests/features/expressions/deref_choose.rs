//! feature: A reference returned from a function, `pick(c, &a, &b) -> &i16`, then read
//! section: Dereference operator
//! note: Wrong answer: reading through the reference the function returns gets the wrong cells (844 and 100 for 40 and 50).
//! input: 40 50
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pick<'a>(c: bool, a: &'a i16, b: &'a i16) -> &'a i16 { if c { a } else { b } }
pub fn main() {
    let a = inp();
    let b = inp();
    out(*pick(true, &a, &b));
    out(*pick(false, &a, &b));
}
