//! feature: A function returning `!` (it panics), called on one branch
//! section: Never type
//! disposition: easy
//! note: A call to a function that never returns is an ordinary call with nothing after it; `panic!()` itself already works.
//! input: 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn fail() -> ! {
    panic!()
}
pub fn main() {
    loop {
        let a = inp();
        if a == 0 {
            fail();
        }
        out(a);
    }
}
