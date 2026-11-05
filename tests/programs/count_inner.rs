#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn id(v: i16) -> i16 {
    v
}

/// `for _ in 0..m` counted down in place, m set in the counter's own block, which the outer loop comes back
/// to: m is set anew before each run (as a larger program's flight/mix is, which the first version of this left counting up).
pub fn main() {
    let mut total = 0;
    for round in 0..3 {
        let m = id(round) + 1;
        for _ in 0..m {
            total += 1;
        }
    }
    out(total);
}
