//! feature: A closure stored in a struct field and called from there
//! section: Closure expressions
//! disposition: demand
//! note: Each closure is its own type, so the struct holding one is generic, compiled once per closure.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Op<F: Fn(i16) -> i16> { f: F, n: i16 }
impl<F: Fn(i16) -> i16> Op<F> {
    #[inline(never)]
    fn run(&self) -> i16 { (self.f)(self.n) }
}
pub fn main() {
    let k = inp();
    let op = Op { f: move |a| a * 2 + k, n: inp() };
    out(op.run());
}
