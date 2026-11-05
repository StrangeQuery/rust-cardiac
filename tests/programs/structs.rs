#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Structs: built, copied, a field changed, passed by value and returned. Each field is a cell of its own.
#[derive(Clone, Copy)]
pub struct Point { pub x: i16, pub y: i16, pub z: i16 }

#[inline(never)]
fn print(p: Point) { out(p.x); out(p.y); out(p.z); }

#[inline(never)]
fn diagonal(v: i16) -> Point { Point { x: v, y: v, z: v } }

#[inline(never)]
fn seven() -> i16 { 7 }

pub fn main() {
    let a = Point { x: seven(), y: 2, z: 3 };
    let mut c = a;
    c.y += a.x;
    print(c);
    print(diagonal(c.z));
}
