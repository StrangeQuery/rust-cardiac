#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `checked_add` and its kin past a card: on CARDIAC an i16 is a card, -999 to 999, so a sum past it is `None`,
/// where natively it would be `Some` up to 32767. Checked against its expected output, as card_overflow.rs is.
pub fn main() {
    let mut k: i16 = 0;
    while k < 3 {
        let v = 985 + 5 * k;
        out(v.checked_add(9).unwrap_or(-1));
        out((-v).checked_sub(9).unwrap_or(-2));
        k += 1;
    }
}
