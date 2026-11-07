//! feature: `let mut x = 0;` whose type comes from a later use
//! section: Let statements
//! input: 7 -3 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut count = 0;
    for _ in 0..3 {
        if inp() > 0 {
            count += 1;
        }
    }
    out(count);
}
