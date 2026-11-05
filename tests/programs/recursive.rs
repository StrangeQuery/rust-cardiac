#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

pub fn down(n: u8) { out(n); if n > 0 { down(n - 1); } }

pub fn main() { down(3); }
