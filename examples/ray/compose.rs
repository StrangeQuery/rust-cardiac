//! The ball laid over one row of the background.
//!
//! Input: where the ball's pixels start on the row and how many there are (from `span`), the ball's colour code for
//! each of them, then the row's 96 background codes (from `sky` or `floor`). Output: the row's 96 codes with the
//! ball over the background. The program comes in two phases: the first reads the ball's codes into an array, and
//! the second, loaded over the first's code, reads the row.
use cardiac::*;

fn main() {
    let mut ball = [0i16; 29]; // at most 29 pixels: radius 14
    let mut before = inp();
    let n = inp() as usize;
    let mut i = 0;
    while i < n && i < 29 {
        ball[i] = inp();
        i += 1;
    }
    overlay();
    let mut next = 0;
    for _ in 0..96 {
        let background = inp();
        if before > 0 {
            before -= 1;
            out(background);
        } else if next < i {
            out(ball[next]);
            next += 1;
        } else {
            out(background);
        }
    }
}
