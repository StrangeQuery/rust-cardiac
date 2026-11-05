#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Every ordering of two values, an array element on the right and a local on the left, with negatives and equal pairs: prints 1 or 0
/// for each. An array element comes through the accumulator, so this exercises the mirrored comparisons (`x < y`
/// as `y - x - 1` not negative) and the parked subtrahend.
pub fn main() {
    let values: [i16; 5] = [-7, 0, 3, 3, 900];
    let mut i: usize = 0;
    while i < 5 {
        let mut j: usize = 0;
        while j < 5 {
            let left = values[i];
            out(if left < values[j] { 1 } else { 0 });
            out(if left <= values[j] { 1 } else { 0 });
            out(if left > values[j] { 1 } else { 0 });
            out(if left >= values[j] { 1 } else { 0 });
            j += 1;
        }
        i += 1;
    }
}
