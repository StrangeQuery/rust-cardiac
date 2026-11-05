#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn id(v: i16) -> i16 {
    v
}

/// `if r.contains(&c)`: the false way's `false`, stored for a test it then jumps past, is a dead store and
/// goes; the value of a contains read later is kept.
pub fn main() {
    let mut c = -2;
    while c < 40 {
        if (0..36).contains(&c) {
            out(c);
        }
        let inside = (5..=7).contains(&id(c));
        out(inside as i16);
        c += 9;
    }
}
