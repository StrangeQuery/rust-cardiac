//! feature: `let Some(x) = o;` is refused by rustc (refutable pattern in `let`)
//! section: Refutability
//! disposition: never
//! note: rustc itself rejects a refutable pattern in a plain `let` (E0005), so this builds nowhere; `let else` and `if let` are the ways round it.
//! note: This row shows Rust's own rule: rustc rejects a refutable pattern in a plain `let` (E0005), so it builds nowhere; `let else` and `if let` are the ways round it.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let o: Option<i16> = if inp() > 0 { Some(1) } else { None };
    let Some(x) = o;
    out(x);
}
