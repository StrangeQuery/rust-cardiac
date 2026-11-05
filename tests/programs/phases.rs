#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

/// The end of a phase: the next phase's code loads over this one's.
#[inline(never)]
pub fn overlay() {}

/// Two phases: the first fills an array of structs through a helper that prints as it goes, the second reads it
/// back. Inlined, the helper's blocks come after the rest of main in rustc's numbering, after the overlay().
#[derive(Clone, Copy)]
pub struct Point { pub x: i16, pub y: i16, pub z: i16 }

#[inline]
fn step(c: i16) -> Point {
    let x = c + 1;
    out(x);
    let y = x + c;
    out(y);
    Point { x, y, z: y - 3 }
}

pub fn main() {
    let mut points = [Point { x: 0, y: 0, z: 0 }; 4];
    let mut n = 0;
    while n < 4 {
        points[n] = step(n as i16);
        n += 1;
    }
    overlay();
    for i in 0..6 {
        let p = points[(i * 3) % 4];
        out(p.x);
        out(p.z);
    }
}
