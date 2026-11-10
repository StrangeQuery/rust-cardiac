use cardiac::*;

/// The 3D tic-tac-toe of the tables-by-tens tip, a layer to a row: each layer's squares numbered 1 to 9 as on a
/// phone's keypad. Reads a layer and a square, puts an X there, and prints 1 if that fills the pillar, else 0.
pub fn main() {
    let mut b = [
        [1i16, 0, 0, 0, 1, 0, -1, 0, 0],
        [0, -1, 1, 0, 1, 0, 0, 0, -1],
        [0, 0, 0, 0, 0, 0, 1, 0, 0],
    ];
    let z = inp() as usize;
    let s = inp() as usize;
    b[z][s - 1] = 1;
    out(if b[0][s - 1] + b[1][s - 1] + b[2][s - 1] >= 3 {
        1
    } else {
        0
    });
}
