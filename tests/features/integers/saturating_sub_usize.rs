//! feature: `a.saturating_sub(b)` on `usize`
//! section: Saturating
//! input: 7 3 3 7 5 5 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp() as usize;
        let b = inp() as usize;
        if a == 0 && b == 0 {
            break;
        }
        out(a.saturating_sub(b) as i16);
    }
}
