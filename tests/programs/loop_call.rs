#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Kept a call: each loop below tests it at its top.
#[inline(never)]
fn three() -> i16 {
    3
}

/// Loops whose condition starts with a call. A call comes back to the cell after its JMP, so the jump back to the
/// top of the loop must not be threaded straight to the call: it would come back past the loop's test.
pub fn main() {
    let mut j = 0;
    while j < three() {
        j += 1;
    }
    out(j);
    for i in 1..=three() {
        out(i);
    }
}
