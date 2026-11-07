//! feature: `let mut x = e;` and later assignments
//! section: Let statements
//! input: 7 3 -5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut total = 0;
    loop {
        let a = inp();
        if a == 0 {
            break;
        }
        total = total + a;
        out(total);
    }
}
