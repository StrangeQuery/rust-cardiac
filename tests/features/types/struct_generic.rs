//! feature: A generic struct `Pair<T>` with `T = i16` and `T = u8`
//! section: Struct types
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
struct Pair<T> {
    a: T,
    b: T,
}
pub fn main() {
    let x = inp();
    let p: Pair<i16> = Pair { a: x, b: x + 1 };
    let q: Pair<u8> = Pair { a: U[inp() as usize], b: 5 };
    out(p.a + p.b);
    out((q.a + q.b) as i16);
}
