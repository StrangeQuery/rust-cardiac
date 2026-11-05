//! The six comparisons of two numbers off cards, each printed as 1 or 0.
use cardiac::*;

fn main() {
    let x = inp();
    let y = inp();
    out(if x < y { 1 } else { 0 });
    out(if x <= y { 1 } else { 0 });
    out(if x > y { 1 } else { 0 });
    out(if x >= y { 1 } else { 0 });
    out(if x == y { 1 } else { 0 });
    out(if x != y { 1 } else { 0 });
}
