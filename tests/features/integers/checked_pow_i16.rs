//! feature: `a.checked_pow(3)` on `i16`
//! section: Checked
//! disposition: demand
//! note: A loop of checked multiplies; rustc calls it at both opt-levels.
//! input: 2 -3 9 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        match a.checked_pow(3) {
            Some(v) => out(v as i16),
            None => out(-1),
        }
        if a == 0 {
            break;
        }
    }
}
