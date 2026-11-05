#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A derived `==` and `!=` of a C-like enum: at opt-level 1 and s rustc calls the impl's `eq`, by reference,
/// the variant compared with a promoted constant (`const main::promoted[0]`, `&Colour::Green`); `!=` calls
/// PartialEq's own `ne`, which is `!eq`. The shared references are passed as the values they read.
#[derive(Clone, Copy, PartialEq)]
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

pub fn main() {
    let mut c = Colour::Red;
    let mut i = 0;
    while i < 5 {
        out(if c == Colour::Green { 1 } else { 0 });
        out(if c != Colour::Blue { 1 } else { 0 });
        c = next(c);
        i += 1;
    }
}
