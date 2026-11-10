#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// A shaded sphere a row at a time: the square root kept as it goes across the row, a step up or down when the
/// square passes it, and the light carried in hundreds and the rest. Must not grow past 94 cells.
pub fn main() {
    let count = inp();
    for _ in 0..count {
        let mut left = inp();
        let mut x = inp();
        let mut root = inp();
        let mut over = inp();
        let mut hundreds = inp();
        let mut rest = inp();
        loop {
            out(hundreds);
            out(rest);
            left -= 1;
            if left == 0 {
                break;
            }
            // the next pixel: the square under the root less 2x + 1, the light 50 less
            over -= 2 * x + 1;
            x += 1;
            let mut change = -50;
            while over > 2 * root {
                over -= 2 * root + 1;
                root += 1;
                change += 62;
            }
            while over < 0 {
                root -= 1;
                over += 2 * root + 1;
                change -= 62;
            }
            rest += change;
            while rest >= 100 {
                rest -= 100;
                hundreds += 1;
            }
            while rest < 0 {
                rest += 100;
                hundreds -= 1;
            }
        }
    }
}
