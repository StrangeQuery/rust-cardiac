//! feature: hygiene: a macro's own `let` doesn't capture the caller's name
//! section: Macros by example
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! plus_ten {
    ($e:expr) => {{
        let x = 10;
        $e + x
    }};
}
pub fn main() {
    let x = inp();
    let y = inp();
    out(plus_ten!(x * 2));
    out(x + y);
}
