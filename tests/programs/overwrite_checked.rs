#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn card(v: i16) -> i16 {
    v
}

/// main's opening blocks: a checked op stores its value straight into v from the block of its op (its success
/// block's move out of the pair became a nop), so v is written before `v = 0` there, which made that constant v's
/// initial value on the tape instead of a store: the op's value was printed.
#[allow(unused_assignments)]
pub fn main() {
    let x = card(1);
    let mut v = 1 + x;
    v = 0;
    out(v);
    let mut w = x - 7;
    w = 4;
    out(w);
    let mut p = x * 3;
    p = -5;
    out(p);
    out(x);
}
