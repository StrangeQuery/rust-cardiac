//! Division under promises, joined with &&: the promises drop the checks, so the numbers off the cards
//! must keep them (the fuzzer's inputs do).
use cardiac::*;

#[inline(never)]
fn ratio(a: i16, b: i16) -> i16 {
    // SAFETY: the fuzzer gives a from 1 and b from 0
    unsafe { assume(a >= 1 && b >= 0) };
    b / a
}

fn main() {
    let a = inp();
    let b = inp();
    out(ratio(a, b));
    // SAFETY: as above, and b is at most 999
    unsafe { assume(a >= 1 && b >= 0 && b < 1000) };
    out(b % a);
}
