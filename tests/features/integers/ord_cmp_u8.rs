//! feature: `a.cmp(&b)` on `u8`, matched on `Ordering`
//! section: Comparison and Ord
//! disposition: easy
//! note: As for `cmp` on `i16`.
//! input: 1 2 3 4 5 6 7 7 6 1 4 3 0 7 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = U[i as usize];
        let b = U[inp() as usize];
        out(match a.cmp(&b) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        });
    }
}
