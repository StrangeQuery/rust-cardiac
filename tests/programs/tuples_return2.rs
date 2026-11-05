#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Tuples returned by a function that isn't inlined: a triple of an i16, a u8 and a bool, and an (i16, bool),
/// the shape of an overflow-checked op's pair, which stays a tuple where an op makes it.
#[inline(never)]
fn three(v: i16) -> (i16, u8, bool) {
    (v, 7, v > 20)
}

#[inline(never)]
fn checked(v: i16) -> (i16, bool) {
    (v, v < 5)
}

pub fn main() {
    let (a, b, c) = three(30);
    out(a);
    out(b as i16);
    out(c as i16);
    let (d, e) = checked(2);
    out(d);
    out(e as i16);
}
