#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn id(v: i16) -> i16 {
    v
}

/// `for _ in 0..n` counted down in place where n is set on one arm of an if, and where it is set again before
/// each run of an inner loop: each run must count from the new bound.
pub fn main() {
    let mut total = 0;
    for round in 0..3 {
        let mut b = id(round - 1);
        if b < 0 {
            b = -b;
        }
        for _ in 0..b {
            total += 1;
        }
    }
    out(total);
    let n = id(7);
    let n = if n >= 5 { n - 5 } else { n };
    for _ in 0..n {
        total += 10;
    }
    out(total);
}
