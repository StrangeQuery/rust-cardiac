//! feature: An associated `const` in a trait
//! section: Traits
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Sides { const N: i16; }
struct Tri;
struct Hex;
impl Sides for Tri { const N: i16 = 3; }
impl Sides for Hex { const N: i16 = 6; }
fn total<T: Sides>(k: i16) -> i16 { T::N * k }
pub fn main() {
    let k = inp();
    out(total::<Tri>(k));
    out(total::<Hex>(k));
}
