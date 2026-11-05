#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[derive(Clone, Copy)]
struct Point {
    x: i16,
    y: i16,
}

#[derive(Clone, Copy)]
struct Line {
    a: Point,
    b: Point,
}

#[inline(never)]
fn len(l: Line) -> i16 {
    (l.b.x - l.a.x) + (l.b.y - l.a.y)
}

#[inline(never)]
fn mk(v: i16) -> Line {
    Line {
        a: Point { x: v, y: 0 },
        b: Point { x: 9, y: v },
    }
}

/// A struct whose fields are structs: its cells are its fields' cells, passed, returned, and a field written
/// whole.
pub fn main() {
    let mut l = mk(2);
    out(len(l));
    l.a = l.b;
    out(len(l));
    l.b = Point { x: 0, y: 0 };
    out(len(l));
}
