//! feature: `cfg!(target_arch = ...)`: which architecture the CARDIAC build reports
//! section: Conditional compilation
//! note: The CARDIAC build reports the host's architecture (`aarch64` on this Mac), not one of its own.
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    if cfg!(target_arch = "x86_64") { out(a); }
    if cfg!(target_arch = "aarch64") { out(2 * a); }
    if cfg!(target_arch = "x86") { out(3 * a); }
    if cfg!(target_arch = "wasm32") { out(4 * a); }
    if cfg!(target_arch = "arm") { out(5 * a); }
    if cfg!(target_arch = "riscv32") { out(6 * a); }
    if cfg!(target_arch = "msp430") { out(7 * a); }
    if cfg!(target_arch = "avr") { out(8 * a); }
    out(0);
}
