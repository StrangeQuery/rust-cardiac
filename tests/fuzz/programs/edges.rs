//! A number off a card against constants at a card's ends, one past them, and around zero (the rewrite of one past a card, and the
//! `x > 999` the compiler once looped on), each printed as 1 or 0.
use cardiac::*;

fn main() {
    let x = inp();
    out(if x < 1000 { 1 } else { 0 });
    out(if x > 999 { 1 } else { 0 });
    out(if x >= -999 { 1 } else { 0 });
    out(if x <= -1000 { 1 } else { 0 });
    out(if x > 0 { 1 } else { 0 });
    out(if x <= -1 { 1 } else { 0 });
    out(if 998 < x { 1 } else { 0 });
    out(if x == -999 { 1 } else { 0 });
}
