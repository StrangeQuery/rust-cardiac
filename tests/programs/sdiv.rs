#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Divide and remainder by powers of ten are decimal shifts (SFT truncates toward zero, as Rust does); by a
/// negative constant, the shared routine. Negative numbers on both sides.
#[inline(never)]
fn show(x: i16) { out(x / 10); out(x % 100); out(x / -7); }

pub fn main() { show(-57); show(9); show(-100); show(999); }
