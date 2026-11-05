#![no_std]
#![allow(unconditional_panic)]
#[inline(never)]
pub fn out(_v: i16) {}

const K: i16 = -1;
const T: [i16; 4] = [-1, -7, -998, 21];

/// An index of -1 as usize: usize::MAX, past what a card holds, which only the bounds check reads, and the check
/// fails every time. It is never stored: the program panics there.
pub fn main() {
    out(T[2]);
    out(T[K as usize]);
    out(T[3]);
}
