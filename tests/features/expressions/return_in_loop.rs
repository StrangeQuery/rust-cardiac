//! feature: `return` from inside a loop
//! section: Return expressions
//! input: 3 5 7 9 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn find(t: &[i16; 5], v: i16) -> i16 {
    for i in 0..5 {
        if t[i] == v { return i as i16; }
    }
    -1
}
pub fn main() {
    let t = [inp(), inp(), inp(), inp(), inp()];
    out(find(&t, 7));
    out(find(&t, 4));
}
