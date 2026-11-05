#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Arrays of structs: filled, a field changed through an index, an element passed whole and a field read out.
/// Each field is an array of its own.
#[derive(Clone, Copy)]
pub struct Point { pub x: i16, pub y: i16, pub z: i16 }

#[inline(never)]
fn print(p: Point) { out(p.x); out(p.y); out(p.z); }

#[inline(never)]
fn diagonal(v: i16) -> Point { Point { x: v, y: v, z: v } }

pub fn main() {
    let mut pts = [Point { x: 0, y: 0, z: 0 }; 3];
    pts[1] = diagonal(7);
    pts[2] = Point { x: 1, y: 2, z: 3 };
    let mut i = 0;
    while i < 3 { pts[i].y += 1; print(pts[i]); i += 1; }
    out(pts[1].x);
    pts[0] = pts[2];
    out(pts[0].z);
}
