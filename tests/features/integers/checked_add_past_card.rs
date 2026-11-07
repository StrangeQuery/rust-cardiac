//! feature: `a.checked_add(b)` on `i16` past a card
//! section: Checked
//! disposition: never
//! note: The `None` is CARDIAC's overflow: every `i16` overflows past a card.
//! input: 500 500 -500 -500
//! differs: CARDIAC's `i16` overflows past a card (999), so 1000 is `None` where native Rust has `Some(1000)`.
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    match a.checked_add(b) {
        Some(v) => out(v / 10),
        None => out(-1),
    }
    let c = inp();
    let d = inp();
    match c.checked_add(d) {
        Some(v) => out(v / 10),
        None => out(-1),
    }
}
