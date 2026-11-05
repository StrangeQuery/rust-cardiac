//! The floor's colours across one row: a red and cream checkerboard, darker in the ball's shadow.
//!
//! Input: the six cards `zrow` outputs for the row (Z, the checker row's parity, the starting x and its remainder,
//! (Z - 60)^2 or -1, and d), then the floor's scroll across (0-19) and the ball's x. Output: 96 colour codes, one
//! for each column. The floor's x under a column is floor((column - 48) 16 / d), and it is stepped across by adding
//! 16 to the remainder for each column and carrying into x, so there is no division. A pixel is checker 4 or 5 by
//! the parity of (x + 200 + scroll) / 10 plus the row's, and 2 more (darker) inside the shadow, a circle of radius 12
//! around (ball x, 60). The program comes in two phases: the first reads the eight cards, and the stepping is loaded
//! over that reading.
mod arith;
use arith::*;
use cardiac::*;

fn main() {
    inp(); // Z isn't needed here
    let row_parity = inp();
    let mut x = inp();
    let mut remainder = inp();
    let shadow_dz2 = inp();
    let d = inp();
    let scroll = inp();
    let ball_x = inp();
    overlay();
    for _ in 0..96 {
        let mut code = checker(x + scroll, row_parity);
        if shadow_dz2 >= 0 && in_shadow(x - ball_x, shadow_dz2) {
            code += 2;
        }
        out(code);
        remainder += 16;
        while remainder >= d {
            remainder -= d;
            x += 1;
        }
    }
}

/// The checker's colour, 4 or 5, at floor position x: the parity of (x + 200) / 10, plus the row's.
fn checker(x: i16, row_parity: i16) -> i16 {
    arith::remainder((x + 200) / 10 + row_parity, 2) + 4
}

/// Whether a point dx across from the ball, and dz2 = dz^2 along, is inside its shadow, radius 12. The square of dx
/// is by adding, in a while loop: with debug assertions on, a for loop's range brings checks that don't fit.
fn in_shadow(dx: i16, dz2: i16) -> bool {
    let size = abs(dx);
    if size >= 12 {
        return false;
    }
    let mut r2 = dz2;
    let mut left = size;
    while left > 0 {
        r2 += size;
        left -= 1;
    }
    r2 < 144
}
