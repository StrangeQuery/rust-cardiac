//! feature: `==`, `!=`, `<`, `>`, `<=`, `>=` on `usize`
//! section: Comparison operators
//! input: 3 5 5 5 5 3 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = i as usize;
        let b = inp() as usize;
        out((a == b) as i16);
        out((a != b) as i16);
        out((a < b) as i16);
        out((a > b) as i16);
        out((a <= b) as i16);
        out((a >= b) as i16);
    }
}
