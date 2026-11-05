//! The floor point a bounced ray lands on, and the checker it sees there, reflected in the ball.
//!
//! Input: t (in tenths) from `bounce`, the ray's x and z (out of 98), the pixel's a and c, the ball's x, and the
//! floor's scroll across and in depth. The floor point is (ball x + a + t x / 10, 60 - c + t z / 10). Output: the
//! reflected checker's code, 10 or 11.
mod arith;
use arith::*;
use cardiac::*;

fn main() {
    let t = inp();
    let ray_x = inp();
    let ray_z = inp();
    let a = inp();
    let c = inp();
    let ball_x = inp();
    let scroll_x = inp();
    let scroll_z = inp();
    let tile_x = ((ray_x * t) / 10 + ball_x + a + 220 + scroll_x) / 10;
    let tile = ((ray_z * t) / 10 + 60 - c + scroll_z) / 10 + tile_x;
    out(remainder(tile, 2) + 10);
}
