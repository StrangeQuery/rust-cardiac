//! feature: `ilog10` in a function of the program's own, its result cast, added to and compared
//! section: Powers and roots
//! input: 5 50 500 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn digits(a: i16) -> i16 {
    a.ilog10() as i16 + 1
}
pub fn main() {
    loop {
        let a = inp();
        if a < 0 {
            break;
        }
        let d = digits(a);
        out(d);
        if d >= 2 {
            out(1);
        }
    }
}
