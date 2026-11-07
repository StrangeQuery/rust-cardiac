//! feature: A generic enum of your own (`enum Maybe<T>`)
//! section: Enumerations
//! disposition: easy
//! note: Once rustc fills in `T`, the layout is the same as a non-generic enum's, which already works.
//! input: 1 7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Maybe<T> { Just(T), Nothing }
#[inline(never)]
fn read() -> Maybe<i16> { if inp() == 1 { Maybe::Just(inp()) } else { Maybe::Nothing } }
fn or(m: Maybe<i16>, d: i16) -> i16 { match m { Maybe::Just(v) => v, Maybe::Nothing => d } }
pub fn main() {
    out(or(read(), -1));
    out(or(read(), -1));
}
