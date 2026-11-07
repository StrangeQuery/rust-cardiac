//! feature: `(a..b).contains(&x)`
//! section: Range expressions
//! input: 3 7 5 9 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    loop {
        let x = inp();
        if x < 0 {
            break;
        }
        out((a..b).contains(&x) as i16);
        out((a..=b).contains(&x) as i16);
    }
}
