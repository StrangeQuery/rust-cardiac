//! feature: Integer literals in a `match`
//! section: Literal patterns
//! input: 1 2 3 7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        let r = match v {
            1 => 10,
            2 => 20,
            3 => 30,
            0 => break,
            _ => -1,
        };
        out(r);
    }
}
