#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn shade(l: i16) -> i16 {
    match l {
        233.. => 3,
        124..233 => 2,
        31..124 => 1,
        ..31 => 0,
    }
}

/// Disjoint ranges, highest first: an arm's lower test settles the next arm's upper one, which the range
/// pass drops once a way in it knows can't be taken is left out of the ranges it joins. Each boundary, both sides.
pub fn main() {
    let ls = [-5, 30, 31, 123, 124, 232, 233, 900];
    let mut i = 0;
    while i < 8 {
        out(shade(ls[i]));
        i += 1;
    }
}
