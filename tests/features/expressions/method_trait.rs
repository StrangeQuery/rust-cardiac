//! feature: A trait method, two types implementing it
//! section: Method-call expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Area { fn area(&self) -> i16; }
struct Sq(i16);
struct Rect(i16, i16);
impl Area for Sq { #[inline(never)] fn area(&self) -> i16 { self.0 * self.0 } }
impl Area for Rect { #[inline(never)] fn area(&self) -> i16 { self.0 * self.1 } }
pub fn main() {
    let a = inp();
    out(Sq(a).area());
    out(Rect(a, 5).area());
}
