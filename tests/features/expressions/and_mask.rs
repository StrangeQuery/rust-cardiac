//! feature: `a & m` with `m` one below a power of two (`a & 3`)
//! section: Bitwise and shift operators
//! note: Compiled as `a.rem_euclid(4)`, a subtracting loop.
//! input: 7 -7 12 0 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a & 3);
        if a == -1 {
            break;
        }
    }
}
