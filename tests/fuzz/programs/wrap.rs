//! A sum that passes a card for a moment and is brought straight back: `u + step` can reach 1798, and `wrap`
//! takes 900 away from it at once, so every value the program keeps is on a card. Without overflow checks it must
//! print what native Rust prints (with them, CARDIAC stops at the sum, as it says).
use cardiac::*;

fn wrap(v: i16, n: i16) -> i16 {
    if v < 0 {
        v + n
    } else if v >= n {
        v - n
    } else {
        v
    }
}

fn main() {
    let mut u = inp();
    let a = inp();
    let b = inp();
    let mut step = a - b - b;
    for row in -6..4 {
        if row >= 0 {
            out(u);
            step = -a;
        }
        u = wrap(u + step, 900);
    }
    out(wrap(u + 110, 900));
}
