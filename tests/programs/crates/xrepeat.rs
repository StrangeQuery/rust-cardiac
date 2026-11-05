//! add_times() from the built-ins crate, on a deck: natively additions, on CARDIAC a computed jump into a
//! row of ADD c, entered for the tens and for the units. Deck: how many pairs, then each pair, a card c and a count
//! n (0 to 99); then a card to take 12 times, and two pairs of a card and its weight.
use cardiac::*;

/// Two cards weighed: two calls of add_times from a function of its own.
#[inline(never)]
fn weigh(a: i16, wa: i16, b: i16, wb: i16) {
    out(add_times(a, wa));
    out(add_times(b, wb));
}

fn main() {
    let pairs = inp();
    for _ in 0..pairs {
        let c = inp();
        let n = inp();
        out(add_times(c, n));
    }
    // a constant count: c * 12 in place
    out(add_times(inp(), 12));
    weigh(inp(), inp(), inp(), inp());
}
