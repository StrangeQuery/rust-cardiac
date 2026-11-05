#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `!=` alone on a derived PartialEq enum: at opt-level 1 and s rustc calls core's `ne`, which calls the
/// derived `eq`, so the driver converts that too.
#[derive(Clone, Copy, PartialEq)]
enum Colour {
    Red,
    Green,
}

#[inline(never)]
fn pick(v: i16) -> Colour {
    if v > 0 { Colour::Green } else { Colour::Red }
}

pub fn main() {
    let mut v = -1;
    while v < 2 {
        out(if pick(v) != Colour::Red { 1 } else { 0 });
        v += 1;
    }
}
