#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

struct Point {
    x: i16,
    y: i16,
}

/// A variant that carries a struct: the struct's fields are the enum's cells.
#[inline(never)]
fn find(v: i16) -> Option<Point> {
    if v > 0 {
        Some(Point { x: v, y: -v })
    } else {
        None
    }
}

pub fn main() {
    let mut i = 0;
    while i < 3 {
        match find(i) {
            Some(p) => {
                out(p.x);
                out(p.y);
            }
            None => out(-9),
        }
        i += 1;
    }
}
