//! feature: A block as a value, `let x = { ...; e };`
//! section: Block expressions
//! input: 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let x = {
        let t = a + b;
        t + t
    };
    out(x);
}
