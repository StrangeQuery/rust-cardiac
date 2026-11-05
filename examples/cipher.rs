//! A substitution cipher on digits. The key is the resource `key`, the file examples/resources/key: ten digits,
//! the one that each of 0 to 9 becomes. Each number from 0 to 999 in the input is printed with its digits
//! substituted: with the key 3 1 4 1 5 9 2 6 5 3, 120 becomes 143.
//!
//!     cargo run --bin cipher <<< "120 7 999"
//!
//! The key is loaded with the program (`load_resource("key")`): CARDIAC's loader puts it in memory with the
//! program's own cells, so there is no code to read it. `digit(n, k)` picks digit k out of a cell.
use cardiac::*;

fn main() {
    let key: [i16; 10] = load_resource("key");
    loop {
        let n = inp();
        let mut coded = 0;
        for k in 0..3 {
            coded = 10 * coded + key[digit(n, k) as usize];
        }
        out(coded);
    }
}
