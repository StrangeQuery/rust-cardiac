//! feature: `!b` on `bool`
//! section: Negation operators
//! input: 0 1 -1
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
        out((!(i > 0)) as i16);
    }
}
