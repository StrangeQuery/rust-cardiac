//! The days in a month, as `days` does, but for a deck that only ever holds a month 1 to 12: the program assumes it,
//! so the compiler leaves out the bounds check, and the deck is smaller. Break the assumption and natively it
//! panics ("a broken assumption"); on CARDIAC it is undefined, unless built with debug assertions on (the
//! `checked` profile), where it panics too.
//!
//!     cargo run --bin days_assumed <<< "2"
use cardiac::*;

const DAYS: [i16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn main() {
    let month = inp();
    // SAFETY: the deck holds a month, 1 to 12
    unsafe { assume(month >= 1) };
    unsafe { assume(month <= 12) };
    out(DAYS[(month - 1) as usize]);
}
