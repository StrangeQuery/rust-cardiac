//! feature: `a.div_euclid(b)` on `i16` by a variable: every sign, exact and not, the ends of a card
//! section: Euclidean division
//! input: 10 7 -10 7 10 -7 -10 -7 -7 7 7 -7 -7 -7 -1 7 -1 -7 -1 1 -1 -1 999 1 -999 1 -999 -1 -999 998 -999 -998 999 -998 -998 999 0 5 0 -5 -6 3 -6 -3 -5 3 -5 -3 1 999 -1 999 0 0
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
        out(a.div_euclid(b));
    }
}
