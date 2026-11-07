//! feature: `#[inline(never)]` on a helper called from four places
//! section: Code generation attributes
//! note: Compare the cells column across these three rows: always copies the body into each caller, never keeps one copy.
//! input: 3 50 -7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn f(x: i16) -> i16 {
    let y = if x < 0 { -x } else { x };
    if y > 40 { y - 40 } else { y + y + 1 }
}

pub fn main() {
    let a = inp();
    let b = inp();
    let c = inp();
    out(f(a) + f(b));
    out(f(c));
    out(f(a + c));
}
