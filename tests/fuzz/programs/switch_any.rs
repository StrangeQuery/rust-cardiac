//! A dense match through a jump table (#34), on a value nothing is known of: below, among or above the arms.
use cardiac::*;

fn main() {
    let x = inp();
    let c = inp();
    out(match x {
        -2 => 1,
        -1 => c,
        0 => 2,
        1 => x + c,
        2 => 3,
        3 => -x,
        4 => 4,
        5 => c - x,
        6 => 5,
        7 => 6,
        _ => 9,
    });
}
