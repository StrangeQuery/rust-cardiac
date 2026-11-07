//! feature: `a.checked_sub(b)` on `i16`
//! section: Checked
//! input: 7 3 -7 3 500 -499 0 0
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
        match a.checked_sub(b) {
            Some(v) => out(v as i16),
            None => out(-1),
        }
    }
}
