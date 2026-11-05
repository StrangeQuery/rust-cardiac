//! Voxel terrain, one depth slice at a time: the projection at the heart of a voxel-space landscape renderer, as
//! flight games drew hills in the early 1990s. `voxel.sh` runs it once for every slice, near to far, and puts what
//! it outputs on the screen.
//!
//! Input: the slice's distance d and the camera's height, then for each of the screen's 96 columns the row the
//! horizon crosses it at (0-95), the height-map cell under that column at distance d, as 10 h + c (its height h and
//! its colour c), and the land's height a little to its left and to its right. Output: for each column, the screen
//! row the cell shows at, and its colour code, 10 c plus its light (0-4), from how steeply the land faces the sun,
//! which is low on the left. A point h below the camera shows 20 (camera - h) / d rows below the horizon, held to
//! the 96-row screen; a point above it, that far above. The division is subtraction, a row at a time, which stops
//! at the screen's edge for free.
use cardiac::*;

fn main() {
    let d = inp();
    let camera = inp();
    for _ in 0..96 {
        let horizon = inp();
        let cell = inp();
        let left = inp();
        let right = inp();
        let height = cell / 10;
        let colour = cell % 10;
        let mut row = horizon;
        let below = camera - height;
        if below >= 0 {
            // below the horizon: a row down for each d in 20 (camera - h), stopping at the bottom
            let mut n = below * 20 - d;
            while n >= 0 && row < 96 {
                n -= d;
                row += 1;
            }
        } else {
            // above it: a row up for each d in 20 (h - camera), stopping at the top
            let mut n = -below * 20 - d;
            while n >= 0 && row >= 1 {
                n -= d;
                row -= 1;
            }
        }
        // the light: how much the land rises from left to right, held to -2..2, and moved up to 0..4
        let mut light = right - left + 2;
        if light < 0 {
            light = 0;
        }
        if light > 4 {
            light = 4;
        }
        out(row);
        out(colour * 10 + light);
    }
}
