//! The ball's bounce and the floor's scroll, once a frame. The ball bounces every 24 frames.
//!
//! Input: the frame f (0-119). Output: the ball's height, 14 + 10 (1 - (2u - 1)^2) rounded half up for
//! u = (f mod 24) / 24, which is 14 + 5 (144 - m^2) / 72 for m = f mod 24 - 12; then the floor's scroll across,
//! 3 f mod 20, and in depth, 2 f mod 20.
use cardiac::*;

/// x mod m, by taking m away (x of 0 or more).
fn modulo(mut x: i16, m: i16) -> i16 {
    while x >= m {
        x -= m;
    }
    x
}

fn main() {
    let f = inp();
    let m = modulo(f, 24) - 12;
    let p = (144 - m * m) * 5;
    let mut h = 0;
    let mut r = p + 36;
    while r >= 72 {
        r -= 72;
        h += 1;
    }
    out(14 + h);
    out(modulo(3 * f, 20));
    out(modulo(2 * f, 20));
}
