//! feature: `fn f(Point { x, y }: Point)`
//! section: Destructuring
//! input: 10 3 -4 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Point { x: i16, y: i16 }
#[inline(never)]
fn diff(Point { x, y }: Point) -> i16 { x - y }
pub fn main() {
    for _ in 0..2 {
        let x = inp();
        let y = inp();
        out(diff(Point { x, y }));
    }
}
