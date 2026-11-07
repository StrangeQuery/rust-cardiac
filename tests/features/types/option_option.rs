//! feature: A nested `Option<Option<i16>>`
//! section: Type layout
//! input: 1 5 1 -1 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn make(a: i16, b: i16) -> Option<Option<i16>> {
    if a > 0 {
        if b > 0 { Some(Some(b)) } else { Some(None) }
    } else {
        None
    }
}
pub fn main() {
    let mut n = 0;
    while n < 3 {
        match make(inp(), inp()) {
            Some(Some(v)) => out(v),
            Some(None) => out(0),
            None => out(-1),
        }
        n += 1;
    }
}
