#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A result worked in its own cell: a tagged Option<u8>.
#[inline(never)]
fn small(v: i16) -> Option<u8> {
    if v < 0 {
        return None;
    }
    let mut n: u8 = 0;
    let mut w = v;
    while w > 9 {
        w -= 10;
        n += 1;
    }
    Some(n)
}

pub fn main() {
    let mut v = -2;
    while v < 20 {
        match small(v + v + v) {
            Some(n) => out(n as i16),
            None => out(-5),
        }
        v += 4;
    }
}
