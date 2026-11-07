//! feature: A let chain, `if let ... && ...`
//! section: If and if let expressions
//! edition: 2024
//! note: Let chains are Rust 2024's: the row is built in that edition, natively too.
//! input: 8 3 4 20
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn half(v: i16) -> Option<i16> { if v % 2 == 0 { Some(v / 2) } else { None } }
pub fn main() {
    for _ in 0..4 {
        let a = inp();
        if let Some(h) = half(a) && h > 2 { out(h); } else { out(0); }
    }
}
