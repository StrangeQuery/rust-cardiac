//! feature: `unreachable!()`
//! section: Panics
//! input: 1 2 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let v = match a {
            1 => 10,
            2 => 20,
            _ => unreachable!(),
        };
        out(v);
    }
}
