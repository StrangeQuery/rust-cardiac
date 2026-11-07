//! feature: `#[must_use]` on a function and a type, the value used and dropped with `let _`
//! section: Diagnostic attributes
//! input: 5 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[must_use]
#[inline(never)]
fn twice(a: i16) -> i16 { a + a }
#[must_use]
struct Tok(i16);
fn tok(a: i16) -> Tok { Tok(a) }
pub fn main() {
    let a = inp();
    let _ = twice(a);
    out(twice(a));
    let t = tok(inp());
    out(t.0);
}
