#[inline(never)]
pub fn out(_v: i16) {}

use cardiac::assume;

const DAYS: [i16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Promises from the built-ins crate's assume(), which rustc calls at opt-level 1 rather than inlining it: each is
/// a promise as assert_unchecked is, so the division needs no check for zero or for i16::MIN / -1 and the table no
/// bounds check, and the tape fits.
#[inline(never)]
fn ratio(a: i16, b: i16) -> i16 {
    unsafe { assume(a >= 1 && b >= 0) };
    b / a
}

#[inline(never)]
fn days(month: i16) -> i16 {
    unsafe { assume(month >= 1) };
    unsafe { assume(month <= 12) };
    DAYS[(month - 1) as usize]
}

pub fn main() {
    out(ratio(7, days(2)));
    out(days(12));
}
