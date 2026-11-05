#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A comparison turned over is the opposite comparison: `!(x < y)` and `!(x <= y)` branched on, and
/// `!(x == y)` kept and printed; and one whose comparison is read twice, which stays as it is. (A derived `!=`:
/// tuple_struct_eq.rs.)
#[inline(never)]
fn check(x: i16, y: i16) {
    if !(x < y) {
        out(2);
    }
    if !(x <= y) {
        out(3);
    }
    let ne = !(x == y);
    out(ne as i16 + 10);
    let lt = x < y;
    let ge = !lt;
    out(lt as i16 + 2 * ge as i16 + 20);
}

pub fn main() {
    check(1, 2);
    check(2, 2);
    check(-3, -7);
}
