//! feature: `a.cmp(&b)` on `i16`, matched on `Ordering`
//! section: Comparison and Ord
//! disposition: easy
//! note: `Ord::cmp` is a call at opt-level 1 and a three-way compare (`Cmp`) at opt-level 2; either is two tests on CARDIAC.
//! input: 7 3 -7 3 5 5 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = inp();
        if a == 0 && b == 0 {
            break;
        }
        out(match a.cmp(&b) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        });
    }
}
