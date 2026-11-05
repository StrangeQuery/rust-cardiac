#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `x % k` and `x / k` with k past a card: a card's size is at most 999, so `x % 1000` is x (with x's sign)
/// and `x / 1000` is 0, and 1000, which no card holds, is never needed.
pub fn main() {
    let mut x: i16 = -999;
    while x < 999 {
        out(x % 1000);
        out(x / 1000);
        out(x % -2000);
        x += 333;
    }
}
