//! feature: `cfg!(target_pointer_width = "16")`, `"32"`, `"64"`
//! section: Conditional compilation
//! note: The CARDIAC build reports the host's target, so `usize` is 64 bits to rustc on a 64-bit host.
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    if cfg!(target_pointer_width = "16") { out(16 * a); }
    if cfg!(target_pointer_width = "32") { out(32 * a); }
    if cfg!(target_pointer_width = "64") { out(64 * a); }
}
