#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Nothing known of the value: a test below the arms and one above, and negative arms among them.
#[inline(never)]
fn squares(d: i16) -> i16 {
    match d {
        -3 => 9,
        -2 => 4,
        -1 => 1,
        0 => 0,
        1 => d,
        2 => 4,
        3 => 9,
        4 => d + d,
        5 => d - 1,
        6 => 36,
        _ => 7,
    }
}

/// A dense match through a jump table, its range not known: values below, among and above the arms.
pub fn main() {
    let mut d = -6;
    while d < 10 {
        out(squares(d));
        d += 1;
    }
}
