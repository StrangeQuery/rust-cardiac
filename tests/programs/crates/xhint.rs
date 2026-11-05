#[inline(never)]
pub fn out(_v: i16) {}

/// core::hint::assert_unchecked of a false condition, in a package built with debug assertions: checked at opt-level
/// 1 too, where it is a call of core's, so cargo's profile has to say debug assertions are on.
#[inline(never)]
fn half(a: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(a >= 0) };
    a / 2
}

pub fn main() {
    out(half(8));
    out(half(-8));
}
