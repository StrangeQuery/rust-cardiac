//! feature: `concat!` and `stringify!`, measured with `.len()`
//! section: Other core macros
//! disposition: demand
//! note: The compiler has no string data yet; the length of a literal is a constant it could fold.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let s = concat!("ab", "cde", 12);
    let t = stringify!(a + 1);
    out(a + s.len() as i16);
    out(t.len() as i16);
}
