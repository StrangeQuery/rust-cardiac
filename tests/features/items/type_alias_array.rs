//! feature: A type alias for an array and a tuple
//! section: Type aliases
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
type Row = [i16; 3];
type Pair = (i16, i16);
#[inline(never)]
fn ends(r: &Row) -> Pair { (r[0], r[2]) }
pub fn main() {
    let r: Row = [inp(), inp(), inp()];
    let p = ends(&r);
    out(p.0 + p.1);
}
