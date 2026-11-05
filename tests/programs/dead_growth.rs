#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A loop that grows a variable nothing reads, past a card at its end: it goes, updates and all, so the range
/// analysis never follows it (it gave up on it, "did not settle": the low end crept down through the small
/// constants one round at a time). Without overflow checks.
#[inline(never)]
fn grow(n: i16) {
    // SAFETY: main passes 0 to 9
    unsafe { core::hint::assert_unchecked(n > -1 && n <= 9) };
    out(12);
    let mut _grown: i16 = 15;
    out(-3);
    let mut k: i16 = 0;
    while k < n {
        if 7 <= k * 4 {
            _grown += 998 - k;
        }
        k += 1;
    }
}

pub fn main() {
    grow(9);
    grow(0);
}
