#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Arrays set from constants after a branch, in a loop, and in a function called more than once: set once and
/// written by nothing else, each is its cells' initial value on the tape. One that is written after
/// it is set is still set each time it is reached.
#[inline(never)]
fn lookup(i: usize, flip: bool) -> i16 {
    if flip {
        out(-1);
    }
    let squares: [i16; 4] = [0, 1, 4, 9];
    squares[i]
}

pub fn main() {
    let mut k: usize = 0;
    while k < 4 {
        out(lookup(k, k == 2));
        let mut counts: [i16; 2] = [5, 6];
        counts[k / 2] = 0;
        out(counts[0] + counts[1]);
        k += 1;
    }
}
