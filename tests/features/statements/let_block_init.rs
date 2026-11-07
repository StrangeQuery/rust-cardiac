//! feature: `let x = { ...; e };` with a block as initialiser
//! section: Let statements
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let x = {
        let t = inp();
        let u = inp();
        t * 2 + u
    };
    out(x);
}
