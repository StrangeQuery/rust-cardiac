#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A struct behind a shared reference is the struct by value: a literal rustc promotes into a constant of its
/// own, `show(&Pair { .. })` (`const main::promoted[0]`), a borrow of a local, and a reference passed on to
/// another function.
struct Pair {
    a: i16,
    b: i16,
}

#[inline(never)]
fn show(p: &Pair) {
    out(p.a);
    out(p.b);
}

#[inline(never)]
fn inner(p: &Pair) -> i16 {
    p.a - p.b
}

#[inline(never)]
fn outer(p: &Pair) -> i16 {
    inner(p) + 1
}

pub fn main() {
    show(&Pair { a: 3, b: 4 });
    let mut k = 0;
    while k < 3 {
        let q = Pair { a: k * 10, b: k + 5 };
        show(&q);
        out(outer(&q));
        k += 1;
    }
}
