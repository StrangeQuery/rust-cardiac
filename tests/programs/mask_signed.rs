#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn card(k: i16) -> i16 {
    k
}

/// `x & m` on an i16, m one below a power of two: x.rem_euclid(m + 1), negative x as two's complement has
/// it; the mask on either side.
pub fn main() {
    let mut v = card(-20);
    while v <= 20 {
        out(v & 7);
        out(15 & v);
        v += 3;
    }
    out(card(-999) & 255);
    out(card(999) & 511);
}
