//! feature: `#[cfg(debug_assertions)]` and `cfg!(debug_assertions)`
//! section: Conditional compilation
//! debug-assertions: on
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    #[cfg(debug_assertions)]
    out(a);
    #[cfg(not(debug_assertions))]
    out(-a);
    out(if cfg!(debug_assertions) { 1 } else { 0 });
}
