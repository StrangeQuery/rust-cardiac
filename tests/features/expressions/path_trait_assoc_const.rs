//! feature: A trait's associated constant (`T::SIZE` in a generic function)
//! section: Path expressions
//! disposition: demand
//! note: At opt-level 1 rustc doesn't inline the generic function, and generic functions are compiled only when rustc inlines them.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Sized3 { const SIZE: i16; }
struct A;
struct B;
impl Sized3 for A { const SIZE: i16 = 3; }
impl Sized3 for B { const SIZE: i16 = 30; }
fn scale<T: Sized3>(x: i16) -> i16 { x + T::SIZE }
pub fn main() {
    let a = inp();
    out(scale::<A>(a));
    out(scale::<B>(a));
}
