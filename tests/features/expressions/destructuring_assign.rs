//! feature: Destructuring assignment: `(a, b) = (b, a)`
//! section: Assignment and compound assignment
//! input: 3 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = inp();
    let mut b = inp();
    (a, b) = (b, a);
    out(a);
    out(b);
    [a, b] = [a + 1, b + 1];
    out(a);
    out(b);
}
