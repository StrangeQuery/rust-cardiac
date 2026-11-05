#[inline(never)]
pub fn out(_v: i16) {}

use cardiac::assume;

const DAYS: [i16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// A broken assumption, built with debug assertions: assume() is then an assert!, not a promise, so the program
/// panics where it does natively rather than read past the table.
#[inline(never)]
fn days(month: i16) -> i16 {
    unsafe { assume(month >= 1) };
    unsafe { assume(month <= 12) };
    DAYS[(month - 1) as usize]
}

pub fn main() {
    out(days(2));
    out(days(13));
}
