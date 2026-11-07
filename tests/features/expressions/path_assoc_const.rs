//! feature: An associated constant (`Point::ORIGIN_X`)
//! section: Path expressions
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Grid;
impl Grid {
    const WIDTH: i16 = 12;
}
pub fn main() {
    let a = inp();
    out(a + Grid::WIDTH);
}
