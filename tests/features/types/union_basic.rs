//! feature: A `union` of `i16` and `u8`, read back through the field written
//! section: Union types
//! note: Reading another field than the one written would reinterpret bytes, which a decimal cell has none of.
//! input: 42 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
union Cell {
    i: i16,
    b: u8,
}
pub fn main() {
    let c = Cell { i: inp() };
    out(unsafe { c.i });
    let d = Cell { b: U[inp() as usize] };
    out(unsafe { d.b } as i16);
}
