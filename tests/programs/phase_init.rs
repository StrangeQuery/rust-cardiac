#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// The end of a phase: the next phase's code loads over this one's.
#[inline(never)]
pub fn overlay() {}

#[inline(never)]
fn id(v: i16) -> i16 {
    v
}

/// The run that opens a later phase stores constants that can be its cells' initial values on the tape, as main's
/// opening blocks do: a loop's counter, a total. But not a local an earlier phase used: its cell holds what
/// that phase left, so the constant must be stored.
pub fn main() {
    let mut k = 0;
    for i in 0..4 {
        k += id(i);
    }
    out(k);
    overlay();
    k = 5;
    let mut total = 0;
    for _ in 0..3 {
        total += k;
    }
    out(total);
}
