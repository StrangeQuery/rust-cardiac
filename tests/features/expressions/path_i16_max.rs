//! feature: `i16::MAX` compared with a value (`a < i16::MAX`)
//! section: Path expressions
//! disposition: easy
//! note: 32767 is past what a card holds, but comparing a value with it works; `i16::MAX - a` is refused on purpose (RCE0009).
//! input: 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let m = i16::MAX;
    out(if a < m { 1 } else { 0 });
}
