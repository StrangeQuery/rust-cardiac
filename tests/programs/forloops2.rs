#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

pub fn main() {
    for i in -2..=1 {
        out(i);
    }
    for k in (1..4).rev() {
        out(k);
    }
    for k in (2..=5i16).rev() {
        if k == 3 {
            break;
        }
        out(k);
    }
}
