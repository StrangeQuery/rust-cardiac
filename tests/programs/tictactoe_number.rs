#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// The 3D tic-tac-toe of the tables-by-tens tip, the move read as one number, its layer then its keypad square (25
/// for layer 2, square 5), as in the tip's "By tens, one number".
pub fn main() {
    let mut b = [
        [1i16, 0, 0, 0, 1, 0, -1, 0, 0],
        [0, -1, 1, 0, 1, 0, 0, 0, -1],
        [0, 0, 0, 0, 0, 0, 1, 0, 0],
    ];
    let n = inp() as usize;
    let (z, s) = (n / 10, n % 10);
    b[z][s - 1] = 1;
    out(if b[0][s - 1] + b[1][s - 1] + b[2][s - 1] >= 3 {
        1
    } else {
        0
    });
}
