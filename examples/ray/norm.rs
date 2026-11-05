//! The ball's surface at one pixel, and the ray that bounces off it. The eye looks straight into the screen, so the
//! surface faces the camera by c = isqrt(196 - a^2 - b^2), and a ray from the eye bounces off in the direction
//! (c a, c b, 98 - c^2), as whole numbers out of 98.
//!
//! Input: the pixel's offset from the ball's centre, a across and b up. Output: c and the bounced ray's x, y and z.
mod arith;
use arith::*;
use cardiac::*;

fn main() {
    let a = inp();
    let b = inp();
    let c = isqrt(196 - a * a - b * b);
    out(c);
    out(a * c);
    out(b * c);
    out(98 - c * c);
}
