//! feature: `matches!(x, pattern if guard)` on an `Option`
//! section: Other core macros
//! input: 7 -3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn nonzero(x: i16) -> Option<i16> {
    if x != 0 { Some(x) } else { None }
}
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        out(matches!(nonzero(a), Some(v) if v > 0) as i16);
    }
}
