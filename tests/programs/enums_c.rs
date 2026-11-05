#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// C-like enums: each is its variant's number, which the driver writes down (the MIR names a variant but
/// doesn't give its number). Numbers in order; a match, a cast and a variant passed and returned.
#[derive(Clone, Copy)]
enum Colour {
    Red,
    Green,
    Blue,
}

#[inline(never)]
fn next(c: Colour) -> Colour {
    match c {
        Colour::Red => Colour::Green,
        Colour::Green => Colour::Blue,
        Colour::Blue => Colour::Red,
    }
}

#[inline(never)]
fn value(c: Colour) -> i16 {
    match c {
        Colour::Red => 10,
        Colour::Green => 20,
        Colour::Blue => 30,
    }
}

pub fn main() {
    let mut c = Colour::Red;
    let mut i = 0;
    while i < 4 {
        out(value(c));
        out(c as i16);
        c = next(c);
        i += 1;
    }
}
