//! Which pixels of a row the ball covers. The ball has a radius of 14 pixels on the screen and its centre is at
//! column 48 + x and row 34 - height.
//!
//! Input: the row y and the ball's x and height. Output: the column the ball's pixels start at on this row and how
//! many there are, those with a^2 + b^2 < 196 (or 0 and 0 if none); then b, the row's height above the centre, and
//! each covered pixel's a, how far across it is from the centre. The square is repeated subtraction, and the half
//! width is a square root, by taking away odd numbers.
mod arith;
use arith::*;
use cardiac::*;

fn main() {
    let y = inp();
    let ball_x = inp();
    let ball_height = inp();
    let b = 34 - ball_height - y;
    let distance = abs(b);
    if distance >= 14 {
        out(0);
        out(0);
        return;
    }
    let mut n = 195;
    for _ in 0..distance {
        n -= distance;
    }
    let half_width = isqrt(n);
    out(48 + ball_x - half_width);
    out(half_width + half_width + 1);
    out(b);
    for a in -half_width..=half_width {
        out(a);
    }
}
