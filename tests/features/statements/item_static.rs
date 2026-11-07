//! feature: a `static` declared inside a function
//! section: Item declarations
//! disposition: demand
//! input: 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    static TABLE: [i16; 3] = [10, 20, 30];
    for _ in 0..2 {
        let i = inp() as usize;
        out(TABLE[i]);
    }
}
