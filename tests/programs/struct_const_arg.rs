#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A struct literal passed by value, which rustc folds into one constant: `field(const Pair {{ a: 25_i16,
/// b: 200_i16 }})` passes the constant's fields, as a struct built from variables passes its cells. The struct is
/// built nowhere else, so its fields are learned from the constant itself.
struct Pair {
    a: i16,
    b: i16,
}

#[inline(never)]
fn field(p: Pair) {
    out(p.a);
    out(p.b);
}

#[inline(never)]
fn sum(p: Pair) -> i16 {
    p.a + p.b
}

pub fn main() {
    field(Pair { a: 25, b: 200 });
    field(Pair { a: -7, b: 0 });
    out(sum(Pair { a: 3, b: 4 }));
}
