//! feature: `==`, `!=`, `<`, `>` on `bool`
//! section: Comparison operators
//! input: 0 0 0 1 1 0 1 1 -1
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
        let a = i > 0;
        let b = inp() > 0;
        out((a == b) as i16);
        out((a != b) as i16);
        out((a < b) as i16);
        out((a > b) as i16);
    }
}
