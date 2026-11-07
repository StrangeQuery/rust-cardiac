//! feature: `if` as a value, `let x = if c { a } else { b };`
//! section: If and if let expressions
//! input: 3 9 7 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        let m = if a > b { a } else { b };
        out(m);
    }
}
