//! feature: `Option<&i16>`, whose `None` is the null pointer
//! section: Type layout
//! disposition: demand
//! input: 4 -4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pick(t: &[i16; 2], k: i16) -> Option<&i16> {
    if k > 0 { Some(&t[1]) } else { None }
}
pub fn main() {
    let t = [10, 20];
    let mut n = 0;
    while n < 2 {
        match pick(&t, inp()) {
            Some(v) => out(*v),
            None => out(-1),
        }
        n += 1;
    }
}
