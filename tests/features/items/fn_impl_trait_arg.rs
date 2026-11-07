//! feature: `impl Trait` in argument position
//! section: Functions
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 4 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Area { fn area(&self) -> i16; }
struct Sq(i16);
struct Rect(i16, i16);
impl Area for Sq { fn area(&self) -> i16 { self.0 * self.0 } }
impl Area for Rect { fn area(&self) -> i16 { self.0 * self.1 } }
fn show(s: impl Area) { out(s.area()); }
pub fn main() {
    let a = inp();
    let b = inp();
    show(Sq(a));
    show(Rect(a, b));
}
