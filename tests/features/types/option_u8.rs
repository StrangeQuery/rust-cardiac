//! feature: `Option<u8>`
//! section: Type layout
//! input: 4 -4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
#[inline(never)]
fn make(k: i16) -> Option<u8> {
    if k > 0 { Some(U[k as usize]) } else { None }
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
