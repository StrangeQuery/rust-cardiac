#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
fn card(v: i16) -> i16 { v * 3 - 20 }

/// A range's `contains`: `a <= x`, then `x < b` (`x <= b` for `..=`). A constant `1..=13` (rustc builds it
/// with `RangeInclusive::new` in a promoted constant of two blocks), a constant `14..27` (a struct literal), and
/// one built as the program runs.
pub fn main() {
    let mut i = 0;
    while i < 15 {
        let c = card(i);
        if (1..=13).contains(&c) { out(1); }
        if (14..27).contains(&c) { out(2); }
        let lo = i - 3;
        if (lo..=i + 4).contains(&c) { out(3); }
        i += 1;
    }
}
