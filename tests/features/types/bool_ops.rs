//! feature: `&`, `|` and `!` on `bool` (not short-circuiting)
//! section: Boolean type
//! input: 1 0 1 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut i = 0;
    while i < 2 {
        let a = inp() > 0;
        let b = inp() > 0;
        out((a & b) as i16);
        out((a | b) as i16);
        out((!a) as i16);
        i += 1;
    }
}
