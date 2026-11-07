//! feature: `x *= a` on `i16`
//! section: Assignment and compound assignment
//! input: 2 3 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut x: i16 = 100;
    loop {
        let a = inp();
        if a == 0 {
            break;
        }
        x *= a;
        out(x);
    }
}
