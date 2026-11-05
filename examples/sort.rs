//! Selection sort: reads ten numbers and prints them smallest first. Each time round it finds the smallest number
//! left, prints it, and puts 999 in its place, the largest a card holds, so it is not found again.
//!
//!     cargo run --bin sort <<< "31 -4 15 9 2 0 -999 8 15 600"
use cardiac::*;

const N: usize = 10;

fn main() {
    let mut xs = [0; N];
    // an index loop: CARDIAC programs index arrays by a range (iterators over arrays are not supported)
    #[allow(clippy::needless_range_loop)]
    for i in 0..N {
        xs[i] = inp();
    }
    for _ in 0..N {
        let mut min = 0;
        for i in 1..N {
            if xs[i] < xs[min] {
                min = i;
            }
        }
        out(xs[min]);
        xs[min] = 999;
    }
}
