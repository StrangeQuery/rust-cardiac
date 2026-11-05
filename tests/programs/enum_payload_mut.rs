#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

struct Point {
    x: i16,
    y: i16,
}

/// An enum that carries a struct, behind a `&mut`: a `&mut` of each cell, the tag's and the struct's fields'.
#[inline(never)]
fn step(o: &mut Option<Point>) {
    match o {
        Some(p) => p.x = p.y,
        None => *o = Some(Point { x: 1, y: 2 }),
    }
}

pub fn main() {
    let mut o = None;
    step(&mut o);
    step(&mut o);
    if let Some(p) = o {
        out(p.x);
    }
}
