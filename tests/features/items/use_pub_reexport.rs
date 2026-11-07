//! feature: `pub use` to re-export
//! section: Use declarations
//! input: 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod inner {
    pub fn sq(x: i16) -> i16 { x * x }
}
mod outer {
    pub use crate::inner::sq;
}
pub fn main() {
    out(outer::sq(inp()));
}
