#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[derive(Clone, Copy)]
enum Op {
    Add,
    Sub,
    Twice,
    Neg,
    Zero,
    One,
    Keep,
    Thrice,
}

/// An enum's eight variants.
#[inline(never)]
fn apply(o: Op) -> i16 {
    match o {
        Op::Add => 7,
        Op::Sub => 5,
        Op::Twice => 3,
        Op::Neg => -1,
        Op::Zero => 6,
        Op::One => 4,
        Op::Keep => 2,
        Op::Thrice => 0,
    }
}

/// A dense match on an enum's variants.
pub fn main() {
    out(apply(Op::Add));
    out(apply(Op::Sub));
    out(apply(Op::Twice));
    out(apply(Op::Neg));
    out(apply(Op::Zero));
    out(apply(Op::One));
    out(apply(Op::Keep));
    out(apply(Op::Thrice));
}
