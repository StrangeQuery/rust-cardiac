#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

pub fn down(n: u8) { out(n); if n > 0 { up(n - 1); } }

pub fn up(n: u8) { down(n); out(n); }

pub fn main() { down(3); }
