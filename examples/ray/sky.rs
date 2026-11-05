//! The sky, once for each row of the picture, top to bottom. `ray.sh` runs it first for every row, and it says
//! whether the row is sky or floor. The picture is 96 wide and 48 high, and the horizon is at row 18; the floor rows
//! closer to it than 4 are too fine to draw and are haze instead.
//!
//! Input: the row y (0-47). Output: for a row of sky, 96 copies of its colour code: three bands of blue, rows 0-5,
//! 6-12 and 13-18, then haze (3) for rows 19-21. For a floor row, from 22 down, a single -1, which sends the row on
//! to `zrow`.
use cardiac::*;

fn main() {
    let y = inp();
    if y >= 22 {
        out(-1);
        return;
    }
    let band = match y {
        ..6 => 0,
        6..13 => 1,
        13..19 => 2,
        _ => 3,
    };
    for _ in 0..96 {
        out(band);
    }
}
