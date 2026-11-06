//! Values read with inp() in a function that isn't inlined and returned from it, straight into the result's cells:
//! the caller reads them. Deck: a card for `read`, one for `some`, two for `rect` and two for `point`.
use cardiac::*;

#[inline(never)]
fn read() -> i16 {
    inp()
}

#[inline(never)]
fn some() -> Option<i16> {
    Some(inp())
}

enum Shape {
    Dot,
    Rect(i16, i16),
}

#[inline(never)]
fn rect(w: i16) -> Shape {
    if w == 0 {
        Shape::Dot
    } else {
        Shape::Rect(w, inp())
    }
}

struct Point {
    x: i16,
    y: i16,
}

#[inline(never)]
fn point(x: i16) -> Point {
    Point { x, y: inp() }
}

fn main() {
    out(read());
    match some() {
        Some(v) => out(v),
        None => out(-1),
    }
    match rect(inp()) {
        Shape::Rect(a, b) => out(a - b),
        Shape::Dot => out(0),
    }
    let p = point(inp());
    out(p.x - p.y);
}
