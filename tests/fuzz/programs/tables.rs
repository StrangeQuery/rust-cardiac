//! Constant arrays: one set after a branch in a function called twice, and one written after it is
//! set, so set again each time round. An index off the table panics both ways.
use cardiac::*;

#[inline(never)]
fn lookup(i: i16, flag: i16) -> i16 {
    if flag < 0 {
        out(-1);
    }
    let squares: [i16; 4] = [0, 1, 4, 9];
    squares[i as usize]
}

fn main() {
    let i = inp();
    let flag = inp();
    out(lookup(i, flag));
    out(lookup(flag, i));
    let mut k = 0;
    while k < 2 {
        let mut counts: [i16; 3] = [5, 6, 7];
        counts[(i + k) as usize] = 0;
        out(counts[0] + counts[1] + counts[2]);
        k += 1;
    }
}
