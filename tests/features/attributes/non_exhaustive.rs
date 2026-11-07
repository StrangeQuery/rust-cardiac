//! feature: `#[non_exhaustive]` on a struct, an enum and a variant (within the crate)
//! section: Type system attributes
//! input: 1 4 2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[non_exhaustive]
struct P { x: i16 }
#[non_exhaustive]
enum E { A(i16), #[non_exhaustive] B(i16) }
pub fn main() {
    loop {
        let k = inp();
        if k == 0 { break; }
        let p = P { x: k };
        let e = if k == 1 { E::A(inp()) } else { E::B(p.x) };
        out(match e { E::A(v) => v, E::B(v) => -v });
    }
}
