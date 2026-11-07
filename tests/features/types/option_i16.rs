//! feature: `Option<i16>` returned from a function
//! section: Type layout
//! input: 4 -4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn make(k: i16) -> Option<i16> {
    if k > 0 { Some(k * 3) } else { None }
}
pub fn main() {
    let mut n = 0;
    while n < 2 {
        match make(inp()) {
            Some(v) => out(v),
            None => out(-1),
        }
        n += 1;
    }
}
