#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Indexes a constant away from a row and a column, `t[i + 1][i]`, as the row and column change in a loop; one
/// from usize::MAX that wraps to 0; and then one past the last row, which panics.
pub fn main() {
    let t = [[0i16, 1, 2], [3, 4, 5], [6, 7, 8]];
    let mut i = 0;
    while i < 2 {
        out(t[i + 1][i] + t[i][i + 1]);
        i += 1;
    }
    let k = core::hint::black_box(-1i16) as usize;
    out(t[k.wrapping_add(2)][k.wrapping_add(1)]);
    out(t[i + 1][0]);
}
