#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// C-like enums: given numbers (negative, worked out) and the ones after them; two enums of one name, one in a
/// module, which the MIR tells apart by their paths.
#[derive(Clone, Copy)]
enum Card {
    Low = -5,
    Mid = 40,
    Next,
    High = 3 * 300,
}

#[derive(Clone, Copy)]
enum Colour {
    Red,
    Blue,
}

mod paint {
    #[derive(Clone, Copy)]
    pub enum Colour {
        Matt = 7,
        Gloss,
    }
}

#[inline(never)]
fn pick(n: i16) -> Card {
    if n < 0 {
        Card::Low
    } else if n < 100 {
        Card::Mid
    } else {
        Card::Next
    }
}

#[inline(never)]
fn finish(g: bool) -> paint::Colour {
    if g {
        paint::Colour::Gloss
    } else {
        paint::Colour::Matt
    }
}

pub fn main() {
    out(pick(-1) as i16);
    out(pick(50) as i16);
    out(pick(200) as i16);
    out(Card::High as i16);
    out(finish(false) as i16);
    out(finish(true) as i16);
    out(Colour::Blue as i16 + Colour::Red as i16);
}
