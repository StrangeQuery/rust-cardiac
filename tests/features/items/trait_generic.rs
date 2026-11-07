//! feature: A generic trait (`trait Scale<T>`)
//! section: Traits
//! disposition: easy
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Scale<T> { fn scale(&self, by: T) -> i16; }
struct V(i16);
impl Scale<i16> for V { fn scale(&self, by: i16) -> i16 { self.0 * by } }
impl Scale<bool> for V { fn scale(&self, by: bool) -> i16 { if by { -self.0 } else { self.0 } } }
pub fn main() {
    let v = V(inp());
    out(v.scale(3i16));
    out(v.scale(true));
}
