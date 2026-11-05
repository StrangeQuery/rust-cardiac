#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 5] = [4, 8, 15, 16, 23];

#[inline(never)]
fn two() -> i16 {
    2
}

/// The steps of for loops, up and down, of each type: with debug assertions on, rustc's inlined `next()` checks
/// `unchecked_add(start, 1)`'s precondition, or `unchecked_sub(end, 1)`'s, or goes to `unreachable_unchecked`'s
/// where a signed step overflows. The loop's own test proves each, so they cost nothing: the same code as
/// without debug assertions.
pub fn main() {
    let mut s = 0;
    for i in 0..T.len() {
        s += T[i];
    }
    out(s);
    for i in (0..T.len()).rev() {
        out(T[i]);
    }
    for i in -3..two() {
        out(i);
    }
    for i in (-3i16..=1).rev() {
        out(i);
    }
    for i in 0u8..3 {
        out(i as i16);
    }
    for _ in 0..two() {
        out(0);
    }
}
