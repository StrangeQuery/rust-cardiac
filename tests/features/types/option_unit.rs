//! feature: `Option<()>`, laid out as a `bool`
//! section: Type layout
//! disposition: easy
//! note: `Option<()>` is a `bool`.
//! input: 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn make(k: i16) -> Option<()> {
    if k > 0 { Some(()) } else { None }
}
pub fn main() {
    let mut n = 0;
    while n < 2 {
        match make(inp()) {
            Some(v) => out({ let () = v; 7 }),
            None => out(-1),
        }
        n += 1;
    }
}
