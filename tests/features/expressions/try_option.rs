//! feature: `?` on an `Option` in a function returning `Option`
//! section: The question mark operator
//! input: 5 3 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn half(x: i16) -> Option<i16> { if x > 0 { Some(x / 2) } else { None } }
#[inline(never)]
fn quarter(x: i16) -> Option<i16> { let h = half(x)?; half(h) }
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        match quarter(a * 4) {
            Some(v) => out(v),
            None => out(-1),
        }
    }
}
