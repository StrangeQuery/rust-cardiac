#[inline(never)]
pub fn out(_v: i16) {}

/// Calls into another crate: one rustc keeps as a call, one it inlines at opt-level 2.
pub fn main() {
    let mut v = -4;
    while v < 5 {
        out(kit::arith::twice(v));
        out(kit::arith::diff(v, 2));
        v += kit::STEP;
    }
}
