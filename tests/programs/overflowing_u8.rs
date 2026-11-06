#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// overflowing_add and overflowing_sub on a u8, as rustc inlines them at opt-level 2: the result wraps, as Rust's
/// does, and the flag says so.
#[inline(never)]
fn both(a: u8, b: u8) {
    let (v, o) = a.overflowing_add(b);
    out(v);
    out(o as u8);
    let (v, o) = a.overflowing_sub(b);
    out(v);
    out(o as u8);
}

pub fn main() {
    both(1, 2);
    both(200, 100);
    both(255, 255);
    both(0, 255);
}
