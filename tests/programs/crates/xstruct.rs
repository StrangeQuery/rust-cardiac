#[inline(never)]
pub fn out(_v: i16) {}

use kit::Point;

fn print(p: Point) {
    out(p.x);
    out(p.y);
    out(p.z);
}

/// Struct consts from another crate, a struct made there and returned, and one passed there and back.
pub fn main() {
    print(Point::FAR);
    print(kit::diagonal(7));
    print(kit::diagonal(0));
    print(kit::moved(kit::BASE));
}
