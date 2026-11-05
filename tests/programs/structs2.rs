#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Structs: a struct passed in and a new one returned, round a loop.
#[derive(Clone, Copy)]
pub struct Point { pub x: i16, pub y: i16, pub z: i16 }

#[inline(never)]
fn print(p: Point) { out(p.x); out(p.y); out(p.z); }

#[inline(never)]
fn moved(p: Point, by: i16) -> Point { Point { x: p.x + by, y: p.y + by, z: p.z + by } }

pub fn main() {
    let mut d = Point { x: 40, y: 40, z: 40 };
    let mut k = 0;
    while k < 3 { d = moved(d, k); k += 1; }
    print(d);
}
