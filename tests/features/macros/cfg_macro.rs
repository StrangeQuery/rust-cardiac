//! feature: `cfg!(...)` as a condition
//! section: Other core macros
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    if cfg!(debug_assertions) {
        out(a);
    } else {
        out(a);
    }
    if cfg!(target_pointer_width = "8") {
        out(0);
    } else {
        out(a + 1);
    }
}
