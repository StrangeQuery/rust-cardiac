//! feature: A `static mut` counter
//! section: Static items
//! disposition: easy
//! note: A `static mut` is just a fixed cell, which is what every CARDIAC variable already is.
//! input: 3 4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
static mut COUNT: i16 = 0;
#[inline(never)]
fn tick(n: i16) { unsafe { COUNT += n; } }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        tick(v);
    }
    out(unsafe { COUNT });
}
