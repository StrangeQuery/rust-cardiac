#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[derive(Clone, Copy)]
struct Point {
    x: i16,
    y: i16,
}

#[inline(never)]
fn nudge(p: &mut Point, d: i16) {
    p.x += d;
    p.y -= d;
}

/// A struct behind a `&mut`: a `&mut` of each field, passed on, reborrowed, and taken in a function.
#[inline(never)]
fn twice(p: &mut Point) {
    nudge(p, 1);
    nudge(&mut *p, 2);
}

pub fn main() {
    let mut p = Point { x: 1, y: 1 };
    twice(&mut p);
    let r = &mut p;
    r.x += 10;
    out(p.x);
    out(p.y);
}
