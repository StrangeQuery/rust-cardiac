//! feature: `Option<bool>`, whose `None` fills a niche in `bool`
//! section: Type layout
//! input: 5 -4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn make(k: i16) -> Option<bool> {
    if k > 0 { Some(k > 2) } else { None }
}
pub fn main() {
    let mut n = 0;
    while n < 2 {
        match make(inp()) {
            Some(v) => out(v as i16),
            None => out(-1),
        }
        n += 1;
    }
}
