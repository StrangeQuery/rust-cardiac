//! feature: `#[cold]` on an error path
//! section: Code generation attributes
//! input: 5 -3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[cold]
#[inline(never)]
fn bad(x: i16) -> i16 { -100 + x }
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        out(if a < 0 { bad(a) } else { a + 1 });
    }
}
