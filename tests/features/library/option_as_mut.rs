//! feature: `Option::as_mut`
//! section: Option
//! disposition: demand
//! note: It makes an `Option<&mut i16>`, and an enum's fields must be scalars for now.
//! input: 5 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn maybe(a: i16) -> Option<i16> {
    if a > 0 { Some(a) } else { None }
}
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let mut o = maybe(a);
        if let Some(x) = o.as_mut() {
            *x += 10;
        }
        out(o.unwrap_or(0));
    }
}
