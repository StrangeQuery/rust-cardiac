//! feature: An array field, `s.t[i]`
//! section: Field access expressions
//! input: 2 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct S { n: i16, t: [i16; 4] }
pub fn main() {
    let mut s = S { n: 0, t: [0; 4] };
    let i = inp() as usize;
    s.t[i] = inp();
    s.n += 1;
    out(s.t[i] + s.n);
    out(s.t[0]);
}
