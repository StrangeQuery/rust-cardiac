//! feature: Labelled `break 'outer` and `continue 'outer`
//! section: Loop expressions
//! input: 4 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    'outer: for i in 0..a {
        for j in 0..a {
            if j > i {
                continue 'outer;
            }
            if i + j > b + 1 {
                break 'outer;
            }
            out(i * 10 + j);
        }
    }
}
