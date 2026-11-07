//! feature: A method taking `&mut self`, not inlined
//! section: Implementations
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct C { n: i16, steps: i16 }
impl C { #[inline(never)] fn step(&mut self) { self.n += 3; self.steps += 1; } }
pub fn main() {
    let mut c = C { n: inp(), steps: 0 };
    c.step();
    c.step();
    out(c.n);
    out(c.steps);
}
