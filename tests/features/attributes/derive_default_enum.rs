//! feature: `#[derive(Default)]` on an enum with `#[default]`
//! section: Derive
//! input: 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Default, Clone, Copy)]
enum Lvl { Low, #[default] Mid, High }
pub fn main() {
    let a = inp();
    let d: Lvl = Default::default();
    out(d as i16 + a);
    let _ = (Lvl::Low, Lvl::High);
}
