#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// The niche layout: an Option whose payload is never negative is its payload's cell alone, None -1 in it.
/// Returned from a function, matched, and `unwrap_or(-1)` (the cell as it is). (`== Some(c)` is niche_eq.rs; an
/// Option<u8> keeps its tag, niche_u8.rs.)
#[inline(never)]
fn find(t: &[i16; 5], v: i16) -> Option<i16> {
    let mut i = 0;
    while i < 5 {
        if t[i as usize] == v {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn main() {
    let t = [7, 3, 9, 3, 5];
    let mut v = 2;
    while v < 11 {
        out(find(&t, v).unwrap_or(-1));
        match find(&t, v) {
            Some(i) => out(i + 100),
            None => out(-100),
        }
        v += 1;
    }
}
