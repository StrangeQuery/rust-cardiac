#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: u8) {}

pub fn main() {
    let mut n: u8 = 5;
    loop {
        out(n);
        if n == 0 {
            break;
        }
        n -= 1;
    }
}
