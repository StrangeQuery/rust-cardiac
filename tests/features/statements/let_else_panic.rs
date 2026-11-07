//! feature: let-else whose `else` panics
//! section: Let-else
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn positive(x: i16) -> Option<i16> {
    if x > 0 { Some(x) } else { None }
}
pub fn main() {
    for _ in 0..2 {
        let Some(v) = positive(inp()) else {
            panic!();
        };
        out(v);
    }
}
