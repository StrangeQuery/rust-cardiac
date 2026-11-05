//! The days in a month: reads the month, 1 to 12, and prints how many days it has (28 for February). A month out
//! of range indexes past the table and panics, as it does natively: CARDIAC outputs -999 and halts, and
//! burn says why from the panic map beside the deck, as `cargo run` does.
//!
//!     cargo run --bin days <<< "2"
use cardiac::*;

const DAYS: [i16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn main() {
    let month = inp();
    out(DAYS[(month - 1) as usize]);
}
