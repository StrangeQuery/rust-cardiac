//! feature: Indexing a `static` array
//! section: Path expressions
//! disposition: easy
//! note: A `static` is a fixed place in memory, which every CARDIAC variable already is; a `const` table works today.
//! input: 2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
static T: [i16; 4] = [11, 22, 33, 44];
pub fn main() {
    let i = inp() as usize;
    let j = inp() as usize;
    out(T[i] + T[j]);
}
