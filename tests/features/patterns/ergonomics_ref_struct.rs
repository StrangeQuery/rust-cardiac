//! feature: Match ergonomics: `&Point` matched with `Point { x, y }`
//! section: Binding modes
//! input: 6 2 -3 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Point { x: i16, y: i16 }
#[inline(never)]
fn diff(p: &Point) -> i16 {
    match p {
        Point { x, y } => *x - *y,
    }
}
pub fn main() {
    for _ in 0..2 {
        let p = Point { x: inp(), y: inp() };
        out(diff(&p));
    }
}
