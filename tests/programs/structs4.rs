#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Struct consts, named by path, and a struct result read straight from the function that made it.
#[derive(Clone, Copy)]
pub struct Point { pub x: i16, pub y: i16, pub z: i16 }

impl Point { pub const ORIGIN: Point = Point { x: 0, y: 0, z: 0 }; }

const CORNER: Point = Point { x: 120, y: 245, z: -30 };

#[inline(never)]
fn print(p: Point) { out(p.x); out(p.y); out(p.z); }

#[inline(never)]
fn diagonal(v: i16) -> Point { Point { x: v, y: v, z: v } }

pub fn main() {
    print(Point::ORIGIN);
    print(CORNER);
    let mut pts = [Point::ORIGIN; 3];
    pts[1] = CORNER;
    print(pts[1]);
    let p = diagonal(9);
    out(p.z);
    let mut q = diagonal(CORNER.y - 240);
    q.x += 1;
    print(q);
}
