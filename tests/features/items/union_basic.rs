//! feature: A `union` read through the field it was written by
//! section: Unions
//! disposition: never
//! note: A union's point is reading one type's bits as another, and CARDIAC's cells hold decimal numbers, not bits.
//! input: 12
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
union U { a: i16, b: u8 }
pub fn main() {
    let u = U { a: inp() };
    out(unsafe { u.a });
}
