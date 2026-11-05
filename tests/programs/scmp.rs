#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Signed comparisons, against zero above all (a negative value is not zero), and a match on negative values.
pub fn main() {
    let xs: [i16; 5] = [-3, -1, 0, 1, 2];
    let mut i = 0usize;
    while i < 5 {
        let x = xs[i];
        if x == 0 { out(100); }
        if x != 0 { out(200); }
        if x < 0 { out(300); }
        if x >= 0 { out(400); }
        out(match x { -1 => 7, 0 => 8, _ => 9 });
        i += 1;
    }
}
