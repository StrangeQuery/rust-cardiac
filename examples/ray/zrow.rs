//! One row of the floor, the way old racing games drew a road: how far away the row is, and where its left edge is.
//!
//! Input: the row y (22-47) and the floor's scroll in depth (0-19). The row is d = y - 18 rows below the horizon.
//! The floor is 16 units below the eye and the screen is 60 units away, so the row shows the floor at depth
//! Z = 960 / d. Output: Z; the parity of its checker row, (Z + scroll) / 10; the floor's x at the left edge,
//! floor(-768 / d), and the remainder that goes with it; (Z - 60)^2 if the ball's shadow (radius 12 around Z = 60)
//! reaches this row, else -1; and d. The divisions are repeated subtraction and the square is repeated addition.
mod arith;
use arith::*;
use cardiac::*;

fn main() {
    let d = inp() - 18;
    let scroll = inp();
    let (z, _) = divide(960, d);
    out(z);
    out(remainder((z + scroll) / 10, 2));
    // floor(-768 / d) is -(768 / d) - 1, with d less the remainder left over, unless d divides it
    let (mut whole, mut left) = divide(768, d);
    if left > 0 {
        left = d - left;
        whole += 1;
    }
    out(-whole);
    out(left);
    let dz = abs(z - 60);
    if dz < 12 {
        let mut square = 0;
        for _ in 0..dz {
            square += dz;
        }
        out(square);
    } else {
        out(-1);
    }
    out(d);
}
