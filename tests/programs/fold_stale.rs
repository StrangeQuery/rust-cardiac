#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Linear folding: `x - y` is a temporary worked out before x is assigned again, and read after. Folded into the sum
/// it was worked out a second time, from the new x: 6 for 1 - 0 + 3, not 4.
#[inline(never)]
fn later(mut x: i16, y: i16) {
    let v = (x - y) + {
        x = y + 3;
        x
    };
    out(v);
    out(x);
}

pub fn main() {
    later(1, 0);
    later(5, 2);
    later(-8, 7);
}
