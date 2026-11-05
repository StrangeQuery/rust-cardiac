#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const DAYS: [i16; 4] = [31, 28, 31, 30];

/// A checked op's result cast, as examples/days.rs indexes, `DAYS[(month - 1) as usize]`: with overflow checks
/// the value is left in the op's pair, and the cast reads it there, `(_t.0: i16) as usize`. i16 to usize after - and
/// +, usize to u8 after + (kept mod 256: 1 + 254 is 255, 2 + 254 is 0). The last month, 0, indexes at -1: as usize it
/// is out of bounds, and panics.
#[inline(never)]
fn tests(m: i16) {
    out(DAYS[(m - 1) as usize]);
    out(DAYS[(m + 1) as usize]);
    let i = m as usize;
    out(((i + 254) as u8) as i16);
}

pub fn main() {
    let months: [i16; 3] = [1, 2, 0];
    let mut k: usize = 0;
    while k < 3 {
        tests(months[k]);
        k += 1;
    }
}
