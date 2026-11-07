//! feature: A `&dyn Trait` whose type is chosen at run time
//! section: Trait object types
//! disposition: demand
//! input: 1 3 4 -1 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Shape {
    fn area(&self) -> i16;
}
struct Sq(i16);
struct Rect(i16, i16);
impl Shape for Sq {
    fn area(&self) -> i16 { self.0 * self.0 }
}
impl Shape for Rect {
    fn area(&self) -> i16 { self.0 * self.1 }
}
#[inline(never)]
fn show(s: &dyn Shape) {
    out(s.area());
}
pub fn main() {
    let mut k = 0;
    while k < 2 {
        let c = inp();
        let a = inp();
        let b = inp();
        let sq = Sq(a);
        let r = Rect(a, b);
        let s: &dyn Shape = if c > 0 { &sq } else { &r };
        show(s);
        k += 1;
    }
}
