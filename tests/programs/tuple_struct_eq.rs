#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Derived `==` on a tuple struct, against its constants: at opt-level 1 rustc calls the impl's `eq` with a
/// promoted `&Flag(0_i16)`.
#[derive(Clone, Copy, PartialEq)]
struct Flag(i16);

impl Flag {
    const CLEAR: Flag = Flag(-1);
    const SET: Flag = Flag(0);
}

#[inline(never)]
fn flag_of(v: i16) -> Flag {
    if v > 2 { Flag::SET } else { Flag::CLEAR }
}

pub fn main() {
    let mut v = 0;
    while v < 5 {
        let f = flag_of(v);
        if f == Flag::SET {
            out(v);
        }
        if f != Flag::CLEAR {
            out(10 + v);
        }
        v += 1;
    }
}
