//! feature: A generic struct (`struct Pair<T>`) at two types
//! section: Structs
//! input: 3 9 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct Pair<T> { a: T, b: T }
const U: [u8; 2] = [7, 250];
pub fn main() {
    let x = inp();
    let y = inp();
    let p = Pair { a: x, b: y };
    let q = Pair { a: U[0], b: U[inp() as usize] };
    out(p.b - p.a);
    out((q.b - q.a) as i16);
}
