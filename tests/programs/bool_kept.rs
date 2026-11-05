#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A bool from a comparison tested at once and again later: the branch on it is one compare and TAC, and
/// on the way where it is true and still to be read it must still be stored, or the later test reads a stale value.
#[inline(never)]
fn escape(rr: i16, ii: i16, mut steps: i16) {
    let mut escaped = rr > 800 - ii;
    if !escaped {
        steps += 1;
        escaped = steps > 400 || steps < -400;
    }
    if escaped {
        steps -= 25;
    }
    out(steps);
}

pub fn main() {
    escape(700, 200, 5);
    escape(900, 0, -3);
    escape(100, 200, 5);
    escape(100, 200, 400);
    escape(100, 200, -401);
}
