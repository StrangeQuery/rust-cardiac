#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A shared reference a function returns is an address, and so is the one it was given: the caller reads the
/// value through it.
#[inline(never)]
fn id(a: &i16) -> &i16 {
    a
}

#[inline(never)]
fn pick<'a>(c: bool, a: &'a i16, b: &'a i16) -> &'a i16 {
    if c {
        a
    } else {
        b
    }
}

/// A reference that is only read through stays the value.
#[inline(never)]
fn twice(a: &i16) -> i16 {
    *a + *a
}

pub fn main() {
    let mut a = 3;
    let mut b = -40;
    while a < 9 {
        out(*id(&a));
        out(*pick(a < 5, &a, &b));
        out(twice(&b));
        a += 2;
        b += 7;
    }
}
