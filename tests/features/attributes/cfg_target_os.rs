//! feature: `cfg!(target_os = ...)` and `cfg!(unix)`: which system the CARDIAC build reports
//! section: Conditional compilation
//! note: The CARDIAC build reports the host's system (`macos` and `unix` on this Mac), not one of its own.
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    if cfg!(target_os = "macos") { out(a); }
    if cfg!(target_os = "linux") { out(2 * a); }
    if cfg!(target_os = "windows") { out(3 * a); }
    if cfg!(target_os = "none") { out(4 * a); }
    if cfg!(unix) { out(10 * a); }
    if cfg!(windows) { out(20 * a); }
    out(0);
}
