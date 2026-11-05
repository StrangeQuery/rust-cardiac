#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

/// 2 to the power 60, digit by digit (examples/powers.rs with no overlay()): too big for memory in one piece, so
/// built with --auto-overlay the compiler splits it into phases itself.
const DIGITS: usize = 20;

pub fn main() {
    let mut d = [0i16; DIGITS];
    d[0] = 1;
    for _ in 0..60 {
        let mut carry = 0;
        #[allow(clippy::needless_range_loop)]
        for i in 0..DIGITS {
            let x = d[i] + d[i] + carry;
            (d[i], carry) = if x >= 10 { (x - 10, 1) } else { (x, 0) };
        }
    }
    let mut top = DIGITS - 1;
    while top > 0 && d[top] == 0 {
        top -= 1;
    }
    for i in (0..=top).rev() {
        out(d[i]);
    }
}
