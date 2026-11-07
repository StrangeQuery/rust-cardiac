//! feature: `if let Some(x) = ... else`
//! section: If and if let expressions
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
        if let Some(v) = pos(a) { out(v * 2); } else { out(-1); }
    }
}
