#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A variant that carries an array: the array is one of the enum's cells.
#[inline(never)]
fn trio(v: i16) -> Option<[i16; 3]> {
    if v > 0 {
        Some([v, 5, -v])
    } else {
        None
    }
}

pub fn main() {
    let mut i = 0;
    while i < 3 {
        if let Some(t) = trio(i) {
            out(t[0]);
            out(t[2]);
        }
        i += 1;
    }
}
