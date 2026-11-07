//! feature: A trait's associated function (no `self`): `T::zero()`
//! section: Traits
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Zero { fn zero() -> Self; }
impl Zero for i16 { fn zero() -> i16 { 0 } }
fn sum<T: Zero + core::ops::Add<Output = T> + Copy>(a: T, b: T) -> T { T::zero() + a + b }
pub fn main() {
    let v = inp();
    out(sum(v, v));
}
