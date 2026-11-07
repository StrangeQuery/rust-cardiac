//! feature: `#[derive(PartialEq, Eq)]` on an enum with data: `==`
//! section: Derive
//! disposition: easy
//! note: At opt-level 1 the derived `eq` calls `<&i16 as PartialEq>::eq` on the payloads, which is a plain `==`.
//! input: 1 5 1 5 1 5 1 6 1 5 2 5 2 0 2 9 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(PartialEq, Eq)]
enum E { A(i16), B }
fn make(k: i16, v: i16) -> E { if k == 1 { E::A(v) } else { E::B } }
pub fn main() {
    loop {
        let k1 = inp();
        if k1 == 0 { break; }
        let a = make(k1, inp());
        let k2 = inp();
        let b = make(k2, inp());
        out(if a == b { 1 } else { 0 });
    }
}
