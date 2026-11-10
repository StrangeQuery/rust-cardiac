use cardiac::*;

/// The 3D tic-tac-toe of the tables-by-tens tip, the move read as one number, its layer then its keypad square (25
/// for layer 2, square 5), as in the tip's "By tens, one number".
///
/// The same program with its input promised (`assume`): the input is always a square on the board, so
/// no bounds check can fail.
pub fn main() {
    let mut b = [
        [1i16, 0, 0, 0, 1, 0, -1, 0, 0],
        [0, -1, 1, 0, 1, 0, 0, 0, -1],
        [0, 0, 0, 0, 0, 0, 1, 0, 0],
    ];
    let n = inp() as usize;
    unsafe { assume(n / 10 < 3) };
    unsafe { assume(n % 10 >= 1) };
    let (z, s) = (n / 10, n % 10);
    b[z][s - 1] = 1;
    out(if b[0][s - 1] + b[1][s - 1] + b[2][s - 1] >= 3 {
        1
    } else {
        0
    });
}
