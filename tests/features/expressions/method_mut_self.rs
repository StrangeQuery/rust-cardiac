//! feature: A method taking `&mut self`
//! section: Method-call expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Counter { n: i16 }
impl Counter {
    #[inline(never)]
    fn tick(&mut self) { self.n += 1; }
}
pub fn main() {
    let mut c = Counter { n: inp() };
    c.tick();
    c.tick();
    out(c.n);
}
