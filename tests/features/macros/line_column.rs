//! feature: `line!()` and `column!()`
//! section: Other core macros
//! note: The row prints the distance between two `line!()`s, since the harness's native copy has a longer prelude.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let first = line!() as i16;
    let second = line!() as i16;
    out(a + second - first);
    out(column!() as i16);
}
