//! feature: Type aliases `type Score = i16` and `type Pair<T> = (T, T)`
//! section: Type aliases
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
type Score = i16;
type Pair<T> = (T, T);
#[inline(never)]
fn total(p: Pair<Score>) -> Score {
    p.0 + p.1
}
pub fn main() {
    let p: Pair<Score> = (inp(), inp());
    out(total(p));
}
