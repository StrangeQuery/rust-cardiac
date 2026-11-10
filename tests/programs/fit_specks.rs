#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// Specks in a box that goes round with the viewer: each speck's place from the viewer, wrapped into the box, three
/// times along. Must not grow past 96 cells.
pub fn main() {
    let up = inp();
    let along = inp();
    let count = inp();
    for _ in 0..count {
        let x = inp();
        let mut y = inp() - up + 70;
        if y < 0 {
            y += 140;
        }
        if y >= 140 {
            y -= 140;
        }
        let mut z = inp() - along + 20;
        if z < 0 {
            z += 140;
        }
        if z >= 140 {
            z -= 140;
        }
        for copy in 0..3 {
            out(x);
            out(y - 70);
            out(z - 20 + copy * 140);
        }
    }
}
