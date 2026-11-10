#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// The 3D tic-tac-toe of the tables-by-tens tip, C-style: three layers of 3 by 3 one after another, 27 squares (X is
/// 1, O is -1). Reads a layer, a row and a column, puts an X there, and prints 1 if that fills the pillar (the same
/// square on all three layers), else 0.
pub fn main() {
    let mut b = [
        1i16, 0, 0, 0, 1, 0, -1, 0, 0, //
        0, -1, 1, 0, 1, 0, 0, 0, -1, //
        0, 0, 0, 0, 0, 0, 1, 0, 0,
    ];
    let z = inp() as usize;
    let y = inp() as usize;
    let x = inp() as usize;
    b[9 * z + 3 * y + x] = 1;
    let s = 3 * y + x;
    out(if b[s] + b[9 + s] + b[18 + s] >= 3 { 1 } else { 0 });
}
