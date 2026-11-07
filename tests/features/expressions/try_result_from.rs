//! feature: `?` converting the error with `From` (`Result<_, u8>` into `Result<_, i16>`)
//! section: The question mark operator
//! note: Wrong answer at opt-level 1: the error comes back unconverted (7, not 70), as if `From` weren't called.
//! input: -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Small(u8);
struct Big(i16);
impl From<Small> for Big { fn from(s: Small) -> Big { Big(s.0 as i16 * 10) } }
#[inline(never)]
fn check(x: i16) -> Result<i16, Small> { if x >= 0 { Ok(x) } else { Err(Small(7)) } }
#[inline(never)]
fn twice(x: i16) -> Result<i16, Big> { let v = check(x)?; Ok(v + v) }
pub fn main() {
    match twice(inp()) {
        Ok(v) => out(v),
        Err(Big(e)) => out(e),
    }
}
