#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

/// The end of a phase: the next phase's code loads over this one's.
#[inline(never)]
pub fn overlay() {}

#[inline(never)]
fn seven() -> i16 { 7 }

/// An overlay() only on one side of an if: the loop after it runs in either phase, so each phase has its own copy.
pub fn main() {
    let x = seven();
    out(x);
    if x > 3 {
        overlay();
    }
    let mut i = 0;
    while i < 3 {
        out(x + i);
        i += 1;
    }
}
