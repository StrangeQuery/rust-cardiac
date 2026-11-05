//! The ball's sway from side to side, once a frame: its x, 28 sin of the frame's angle, rounded.
//!
//! Input: 999 sin of the frame's angle, from the table `sine120.txt` (frame f is f 120ths of a turn). Output: the
//! ball's x, 28 t / 999 rounded. There is no division: with |t| = 36 a + b, 28 |t| = 999 a + 9 a + 28 b, so the
//! answer is a, and one more when 9 a + 28 b reaches 500 (never a tie, since 56 t is even and 999 is odd).
use cardiac::*;

fn main() {
    let t = inp();
    let neg = t < 0;
    let mut a = 0;
    let mut b = if neg { -t } else { t };
    while b >= 36 {
        b -= 36;
        a += 1;
    }
    let x = 28 * b - 500 + 9 * a;
    let q = if x >= 0 { a + 1 } else { a };
    out(if neg { -q } else { q });
}
