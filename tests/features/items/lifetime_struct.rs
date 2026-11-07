//! feature: A struct holding a reference (`struct View<'a>`)
//! section: Generic parameters
//! disposition: demand
//! note: A function that returns such a struct is refused (RCE0004: unsupported local type `View<'_>`).
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct View<'a> { t: &'a [i16; 3], at: usize }
impl<'a> View<'a> { fn get(&self) -> i16 { self.t[self.at] } }
pub fn main() {
    let t = [inp(), inp(), inp()];
    for at in 0..3 { let v = View { t: &t, at }; out(v.get() * 2); }
}
