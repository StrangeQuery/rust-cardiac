#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A switch on an element: a bool's and a u8's are never negative, so `== 0` needs no sign test; an i16's
/// can be, and keeps it (-3 must not count as 0).
pub fn main() {
    let flags = [true, false, false, true, false];
    let small: [u8; 5] = [0, 7, 0, 255, 1];
    let signed: [i16; 5] = [0, -3, 4, 0, -1];
    let mut i = 0;
    while i < 5 {
        if flags[i] {
            out(10 + i as i16);
        }
        match small[i] {
            0 => out(20),
            1 => out(21),
            _ => out(29),
        }
        match signed[i] {
            0 => out(30),
            _ => out(39),
        }
        i += 1;
    }
}
