//! feature: `==`, `!=`, `<`, `>`, `<=`, `>=` on `i16`
//! section: Comparison operators
//! input: 7 3 -7 3 7 -3 -7 -3 999 -999 5 5 0 5 500 499 -1 1 0 0
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
        out((a == b) as i16);
        out((a != b) as i16);
        out((a < b) as i16);
        out((a > b) as i16);
        out((a <= b) as i16);
        out((a >= b) as i16);
    }
}
