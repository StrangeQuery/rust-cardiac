//! feature: `x @ Some(_)`
//! section: Identifier patterns
//! input: 5 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pos(v: i16) -> Option<i16> { if v > 0 { Some(v) } else { None } }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        match pos(v) {
            o @ Some(_) => out(o.unwrap_or(0) + 1),
            None => out(-1),
        }
    }
}
