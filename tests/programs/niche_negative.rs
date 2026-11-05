#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// An Option whose payload can be negative keeps its tag cell: the niche's guard, payload >= 0, isn't proved,
/// so the compile is done again without the niche. Some(-1) and None stay apart.
#[inline(never)]
fn less(v: i16) -> Option<i16> {
    if v > -3 {
        Some(v - 2)
    } else {
        None
    }
}

pub fn main() {
    let mut v = -4;
    while v < 3 {
        match less(v) {
            Some(x) => out(x),
            None => out(500),
        }
        out(less(v).unwrap_or(-1));
        v += 1;
    }
}
