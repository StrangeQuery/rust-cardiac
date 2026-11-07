//! feature: `match` on an `Option`
//! section: Match expressions
//! input: 4 -4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pos(x: i16) -> Option<i16> { if x > 0 { Some(x) } else { None } }
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        out(match pos(a) { Some(v) => v, None => -1 });
    }
}
