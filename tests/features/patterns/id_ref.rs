//! feature: `ref x` binding
//! section: Identifier patterns
//! input: 8 -4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Pair { a: i16, b: i16 }
pub fn main() {
    let p = Pair { a: inp(), b: inp() };
    let Pair { ref a, ref b } = p;
    out(*a + *b);
    out(*a);
}
