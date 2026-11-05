#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
fn three() -> i16 {
    3
}

/// Tuple structs set from constants: rustc writes `const Flag(-1_i16)`, as an enum's variant is written, and
/// builds one with `Flag(move _1)`; with no enum of that name they are the struct's, fields `.0`, `.1`. (Derived
/// `==` against such a constant: tuple_struct_eq.rs.)
#[derive(Clone, Copy, PartialEq)]
struct Flag(i16);

impl Flag {
    const CLEAR: Flag = Flag(-1);
    const SET: Flag = Flag(0);
    #[inline(always)]
    fn is_set(self) -> bool {
        self.0 >= 0
    }
}

#[derive(Clone, Copy)]
struct Span(i16, i16);

const WHOLE: Span = Span(-5, 5);

pub fn main() {
    let mut f = Flag::CLEAR;
    let mut s = WHOLE;
    let mut i = 0;
    while i < 4 {
        if i == three() - 1 {
            f = Flag::SET;
            s = Span(s.0 + 1, s.1 - 1);
        }
        if f.is_set() {
            out(s.1 - s.0);
        }
        i += 1;
    }
}
