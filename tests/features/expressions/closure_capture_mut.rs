//! feature: A closure changing a captured variable (`FnMut`)
//! section: Closure expressions
//! disposition: easy
//! note: At opt-level 1 rustc calls the closure rather than inlining it; the call is a method on a struct of its captures.
//! input: 3 4 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut total = 0;
    let mut add = |a: i16| total += a;
    loop {
        let a = inp();
        if a == 0 { break; }
        add(a);
    }
    out(total);
}
