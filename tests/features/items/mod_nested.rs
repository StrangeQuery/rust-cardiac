//! feature: Nested modules and a full path (`crate::a::b::f`)
//! section: Modules
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod a {
    pub mod b {
        pub fn inc(x: i16) -> i16 { x + 1 }
    }
}
pub fn main() {
    let v = inp();
    out(crate::a::b::inc(v));
    out(a::b::inc(v));
}
