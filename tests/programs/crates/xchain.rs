#[inline(never)]
pub fn out(_v: i16) {}

/// This crate's own twice, which isn't kit's: kit's quad must still call kit's.
#[inline(never)]
fn twice(v: i16) -> i16 { v + 1 }

/// A function in another crate that calls another function there.
pub fn main() {
    let mut v = 0;
    while v < 4 {
        out(kit::arith::quad(v));
        out(twice(v));
        v += 1;
    }
}
