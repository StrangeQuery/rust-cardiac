#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// An Option<u8> keeps its tag cell: -1 isn't a u8, and the range pass knows a u8 is 0 to 255, so the niche
/// layout is an i16's only.
#[inline(never)]
fn small(v: u8) -> Option<u8> {
    if v < 200 {
        Some(v + 50)
    } else {
        None
    }
}

pub fn main() {
    let mut v: u8 = 120;
    while v < 240 {
        match small(v) {
            Some(b) => out(b as i16),
            None => out(-2),
        }
        v += 40;
    }
}
