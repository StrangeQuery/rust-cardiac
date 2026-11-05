#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A tuple that crosses a call is a struct whose fields are .0, .1, ...: a pair returned, and passed as an
/// argument. (tuples_return2.rs: a triple, and an (i16, bool).)
#[inline(never)]
fn split(v: i16) -> (i16, i16) {
    (v / 10, v % 10)
}

#[inline(never)]
fn join(p: (i16, i16)) -> i16 {
    p.0 - p.1
}

pub fn main() {
    let mut v = 7;
    while v < 40 {
        let (t, u) = split(v);
        out(t);
        out(u);
        out(join(split(v)));
        v += 13;
    }
}
