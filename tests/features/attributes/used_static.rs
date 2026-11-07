//! feature: `#[used]` on a static the program reads
//! section: Linkage attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[used]
static KEEP: i16 = 42;
pub fn main() {
    let a = inp();
    out(a + KEEP);
}
