#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn id(v: i16) -> i16 {
    v
}

/// `for _ in a..n`, n known only at run time: the loop counts n down, as `.rev()` does. Re-entered by the outer
/// loop, it must count down afresh each time; and an empty range runs no times.
pub fn main() {
    let mut sum = 0;
    for round in 0..4 {
        let n = id(round);
        for _ in 0..n {
            sum += 1;
        }
        for _ in 2..n {
            sum += 10;
        }
    }
    out(sum);
    for _ in 0..id(-3) {
        out(-2);
    }
}
