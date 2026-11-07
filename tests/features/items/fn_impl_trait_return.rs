//! feature: `impl Trait` in return position
//! section: Functions
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Val { fn val(&self) -> i16; }
struct W(i16);
impl Val for W { fn val(&self) -> i16 { self.0 + 1 } }
#[inline(never)]
fn make(x: i16) -> impl Val { W(x) }
pub fn main() {
    let v = make(inp());
    out(v.val());
}
