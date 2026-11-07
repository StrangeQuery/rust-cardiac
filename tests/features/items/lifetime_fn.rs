//! feature: A lifetime-annotated function (`fn pick<'a>(&'a i16, &'a i16) -> &'a i16`)
//! section: Generic parameters
//! note: A wrong answer: the returned reference reads the wrong cell (128 for 8), as in the row for a function returning a reference.
//! input: 3 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn larger<'a>(a: &'a i16, b: &'a i16) -> &'a i16 { if *a > *b { a } else { b } }
pub fn main() {
    let a = inp();
    let b = inp();
    out(*larger(&a, &b));
}
