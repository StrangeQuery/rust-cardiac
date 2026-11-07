//! feature: `PartialEq` implemented by hand
//! section: Traits
//! input: 3 13 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Mod10(i16);
impl PartialEq for Mod10 { fn eq(&self, o: &Mod10) -> bool { self.0 % 10 == o.0 % 10 } }
pub fn main() {
    let a = Mod10(inp());
    let b = Mod10(inp());
    out((a == b) as i16);
    let c = Mod10(inp());
    let d = Mod10(inp());
    out((c != d) as i16);
}
