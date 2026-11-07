//! feature: `file!()` and `module_path!()`, measured with `.len()`
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
    out(a + module_path!().len() as i16);
    out((file!().len() > 0) as i16);
}
