//! feature: `(a..b).nth(n)`
//! section: Iterators
//! disposition: easy
//! note: On a range it is `a + n`, or `None` past the end.
//! input: 2 6 1 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    for _ in 0..2 {
        let n = inp() as usize;
        out((a..b).nth(n).unwrap_or(-1));
    }
}
