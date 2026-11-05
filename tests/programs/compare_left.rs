#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Every ordering of two values, an array element on the left and a local on the right, with negatives and equal pairs: prints 1 or 0
/// for each. An array element comes through the accumulator, so this exercises the mirrored comparisons (`x < y`
/// as `y - x - 1` not negative) and the parked subtrahend.
pub fn main() {
    let values: [i16; 5] = [-7, 0, 3, 3, 900];
    let mut i: usize = 0;
    while i < 5 {
        let mut j: usize = 0;
        while j < 5 {
            let right = values[j];
            out(if values[i] < right { 1 } else { 0 });
            out(if values[i] <= right { 1 } else { 0 });
            out(if values[i] > right { 1 } else { 0 });
            out(if values[i] >= right { 1 } else { 0 });
            j += 1;
        }
        i += 1;
    }
}
