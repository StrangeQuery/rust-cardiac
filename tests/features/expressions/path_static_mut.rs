//! feature: Reading and writing a `static mut`
//! section: Path expressions
//! disposition: easy
//! note: A `static` is a fixed place in memory, which every CARDIAC variable already is.
//! input: 3 4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
static mut TOTAL: i16 = 0;
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        unsafe { TOTAL += a; }
    }
    out(unsafe { TOTAL });
}
