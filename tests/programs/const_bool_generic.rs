#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A `bool` const generic compared with a value is a constant in each instance: `(x < 0) != NEG` is `x < 0` where NEG
/// is false and `x >= 0` where it is true, tested as the comparison itself, with no bool kept in a cell; and the same
/// with `==` and the constant on the left.
#[inline(never)]
fn run<const NEG: bool>(x: i16) {
    out(if (x < 0) != NEG { 1 } else { 2 });
    out(if NEG == (x > 3) { 3 } else { 4 });
}

pub fn main() {
    let vs: [i16; 4] = [-999, 0, 3, 999];
    let mut i = 0;
    while i < 4 {
        run::<false>(vs[i]);
        run::<true>(vs[i]);
        i += 1;
    }
}
