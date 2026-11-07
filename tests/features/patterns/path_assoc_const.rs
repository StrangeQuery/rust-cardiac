//! feature: An associated const as a pattern (`Limits::MAX =>`)
//! section: Path patterns
//! input: 9 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Limits;
impl Limits { const MAX: i16 = 9; }
pub fn main() {
    for _ in 0..2 {
        let v = inp();
        out(match v { Limits::MAX => 1, _ => 0 });
    }
}
