#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Arms that only go on: a `break` or a `continue` straight from the table, and a return.
#[inline(never)]
fn count(limit: i16) -> i16 {
    let mut n = 0;
    let mut k = 0;
    loop {
        k += 1;
        if k > limit {
            break;
        }
        match k {
            0 => break,
            1 => continue,
            2 => n += 2,
            3 => continue,
            4 => n += 4,
            5 => continue,
            6 => return n + 100,
            _ => n += 7,
        }
    }
    n
}

/// A dense match whose arms break, continue and return.
pub fn main() {
    let mut d = -2;
    while d < 9 {
        out(count(d));
        d += 1;
    }
}
