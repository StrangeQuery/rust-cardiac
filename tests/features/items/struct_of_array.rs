//! feature: A struct with an array field
//! section: Structs
//! input: 1 2 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Stack { items: [i16; 4], len: usize }
pub fn main() {
    let mut s = Stack { items: [0; 4], len: 0 };
    for _ in 0..4 { s.items[s.len] = inp(); s.len += 1; }
    while s.len > 0 { s.len -= 1; out(s.items[s.len]); }
}
