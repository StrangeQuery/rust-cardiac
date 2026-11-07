//! feature: An associated `const` in an inherent impl
//! section: Associated items
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Grid;
impl Grid { const W: i16 = 8; fn cell(r: i16, c: i16) -> i16 { r * Self::W + c } }
pub fn main() {
    let r = inp();
    out(Grid::cell(r, 5));
    out(Grid::W);
}
