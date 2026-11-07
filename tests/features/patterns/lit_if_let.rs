//! feature: A literal in `if let` (`if let 3 = v`)
//! section: Literal patterns
//! input: 3 4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        if let 3 = v { out(1); } else { out(0); }
    }
}
