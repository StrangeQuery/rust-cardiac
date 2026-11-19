#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A remainder by 10 is -9 to 9: the negative ones go to the otherwise arm.
#[inline(never)]
fn last_digit(n: i16) -> i16 {
    match n % 10 {
        0 => 50,
        1 => 51,
        2 => 52,
        3 => 53,
        4 => 54,
        5 => 55,
        6 => 56,
        7 => 57,
        8 => 58,
        9 => 59,
        _ => -1,
    }
}

/// A dense match on a remainder, negative and not.
pub fn main() {
    let mut d = -23;
    while d < 24 {
        out(last_digit(d));
        d += 1;
    }
}
