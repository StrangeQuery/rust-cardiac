//! feature: Or-pattern binding the same name (`(x, 0) | (0, x)`)
//! section: Or-patterns
//! input: 5 0 0 6 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let b = inp();
        let r = match (a, b) {
            (x, 0) | (0, x) => x,
            _ => -1,
        };
        out(r);
    }
}
