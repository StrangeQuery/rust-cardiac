#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn card(k: i16) -> i16 {
    k * 10 + 10
}

/// `x & 3` on a loop counter that runs 0..8 is x % 4, not the `b & k` of a b that is 0 or 1.
pub fn main() {
    let mut t = [0; 4];
    for i in 0..4 {
        t[i] = card(i as i16);
    }
    for x in 0..8usize {
        out(t[x & 3]);
    }
}
