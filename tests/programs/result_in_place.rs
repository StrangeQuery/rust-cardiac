#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A result built in a local and moved out at the end is worked in the result's own cell: `Some(t)` with t
/// counted in a loop that calls a function while the result's cell is live. (A tagged Option:
/// result_in_place2.rs; a tuple: result_in_place3.rs.)
#[inline(never)]
fn step(x: i16) -> i16 {
    x / 2 + 3
}

#[inline(never)]
fn count(limit: i16, from: i16) -> Option<i16> {
    if limit < from {
        return None;
    }
    let mut t = 0;
    let mut x = from;
    while x < limit {
        x += step(x);
        t += 1;
    }
    Some(t)
}

pub fn main() {
    let mut v = -2;
    while v < 20 {
        out(count(v, 3).unwrap_or(-1));
        v += 4;
    }
}
