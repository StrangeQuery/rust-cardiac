//! feature: A nested tuple `((i16, i16), i16)`
//! section: Tuple types
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: ((i16, i16), i16) = ((inp(), inp()), inp());
    out((t.0).0 + t.1);
    out((t.0).1 * 2);
}
