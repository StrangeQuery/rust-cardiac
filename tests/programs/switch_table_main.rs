#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A dense match in main, whose arm 4 ends the program as main returns.
pub fn main() {
    let mut k = 0;
    loop {
        k += 1;
        match k {
            0 => out(1),
            1 => out(2),
            2 => out(k),
            3 => out(-k),
            4 => return,
            5 => out(0),
            6 => out(k + k),
            _ => out(5),
        }
    }
}
