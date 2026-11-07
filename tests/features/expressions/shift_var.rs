//! feature: `a << b`, `a >> b`: shifts by a variable
//! section: Bitwise and shift operators
//! disposition: demand
//! input: 3 2 0 0
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
        out(a << b);
        out(a >> b);
    }
}
