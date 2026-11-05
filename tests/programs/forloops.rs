#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

pub fn main() {
    let n: i16 = 4;
    for i in 0..n {
        out(i);
    }
    for i in 0..3 {
        if i == 1 {
            continue;
        }
        for j in i..3 {
            out(10 * i + j);
        }
    }
    for i in 5..2 {
        out(i);
    }
}
