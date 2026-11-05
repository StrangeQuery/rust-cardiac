#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn card(k: i16) -> i16 {
    k
}

/// `x & 3` on a loop counter in a condition with `||`: the element it picks decides, so `t[x & 3]` must be
/// t[x % 4], not t[0] or t[3].
pub fn main() {
    let t = [card(10), card(20), card(30), card(40)];
    for x in 0..8usize {
        out(if x > 5 || t[x & 3] >= 25 { x as i16 } else { -1 });
    }
}
