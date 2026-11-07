//! feature: `compile_error!` stops the build (natively too)
//! section: Other core macros
//! disposition: never
//! note: This row can't build anywhere: it shows rustc rejecting the program before the compiler sees it.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[cfg(not(feature = "never_set"))]
compile_error!("this row is meant not to build");
pub fn main() {
    out(inp());
}
