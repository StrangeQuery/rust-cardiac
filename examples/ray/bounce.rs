//! Where a bounced ray goes. A ray up and to the left catches the light; any other upward ray sees the sky, high or
//! low. A ray going down falls from the height it left the ball at, ball height + b, and meets the floor after
//! t = 10 height / -y tenths of its length, a division by repeated subtraction. Past half a length the floor is lost
//! in haze.
//!
//! Input: the ray's x and y (out of 98), the pixel's b, and the ball's height. Output: a colour code and 0 (the light
//! 13, high sky 8, low sky 9, haze 12), or -1 and t for a ray that reaches the floor, which `hit` finishes.
mod arith;
use arith::*;
use cardiac::*;

fn main() {
    let ray_x = inp();
    let ray_y = inp();
    let b = inp();
    let ball_height = inp();
    match ray_y {
        45.. if ray_x <= -35 => {
            out(13);
            out(0);
        }
        51.. => {
            out(8);
            out(0);
        }
        0..51 => {
            out(9);
            out(0);
        }
        ..0 => {
            let (t, _) = divide((ball_height + b) * 10, -ray_y);
            if t < 5 {
                out(-1);
                out(t);
            } else {
                out(12);
                out(0);
            }
        }
    }
}
