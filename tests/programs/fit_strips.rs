#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

fn abs(v: i16) -> i16 {
    if v < 0 {
        -v
    } else {
        v
    }
}

const CELL: i16 = 700;
const MIDDLES: [i16; 5] = [230, 283, 337, 394, 460];

/// A row of strips across repeating cells of 700: for each of 192 pixels, a step apart (step + fraction / 500), the
/// strip it is on, 31 wide, and how far from its middle; else -5. Its sums pass 999 for a moment (place + step),
/// which the tests straight after read whole from the accumulator. Must not grow past 94 cells.
pub fn main() {
    let step = inp();
    let fraction = inp();
    let mut column = inp(); // 80 c
    let mut place = inp();
    let mut part = inp();
    let over = step - CELL; // a step less a cell: place + step - 700 stays on a card
    for _ in 0..192 {
        let mut code = -5;
        let mut strip = column;
        for s in 0..5 {
            let off = abs(place - MIDDLES[s]);
            if off <= 15 {
                code = strip + off;
            }
            strip += 16;
        }
        out(code);
        part += fraction;
        if part >= 500 {
            part -= 500;
            place += 1;
        }
        place += over;
        if place < 0 {
            place += CELL;
        } else {
            column += 80; // the next card
            if column >= 640 {
                column = 0;
            }
        }
    }
}
