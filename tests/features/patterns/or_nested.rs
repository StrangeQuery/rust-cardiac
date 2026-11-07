//! feature: Nested or-pattern (`Some(1 | 2)`)
//! section: Or-patterns
//! input: 1 2 3 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pos(v: i16) -> Option<i16> { if v > 0 { Some(v) } else { None } }
pub fn main() {
    for _ in 0..4 {
        let v = inp();
        out(match pos(v) { Some(1 | 2) => 1, Some(_) => 2, None => 0 });
    }
}
