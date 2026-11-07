//! feature: `a.cmp(&b).is_lt()` and `.is_ge()` on `i16`
//! section: Comparison and Ord
//! disposition: easy
//! note: As for `cmp`; at opt-level 2 rustc also reads the `Ordering` as an `i8`, which the compiler lacks.
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
        out(a.cmp(&b).is_lt() as i16);
        out(a.cmp(&b).is_ge() as i16);
    }
}
