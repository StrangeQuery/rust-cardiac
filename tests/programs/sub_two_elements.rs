#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
/// A value rustc can't see through, as a card read would be.
#[inline(never)]
pub fn card(v: i16) -> i16 {
    v
}

/// `930 - t[a] - t[b]`, both indexes promised in bounds: the partial difference `930 - t[a]` must be stored before
/// `t[b]` is read through its address, which takes the accumulator.
pub fn main() {
    let mut squares = [0i16; 8];
    for i in 0..8 {
        squares[i] = card(i as i16 * 2 + 40);
    }
    for k in 0..3 {
        let side = card(2 * k - 1).abs().min(7);
        let beyond = card(2 * k + 3);
        unsafe { core::hint::assert_unchecked(side >= 0 && beyond >= 0 && beyond < 8) }
        out(930 - squares[side as usize] - squares[beyond as usize]);
    }
}
