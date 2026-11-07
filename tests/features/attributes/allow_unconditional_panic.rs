//! feature: `#[allow(unconditional_panic)]` over an index rustc knows is out of bounds
//! section: Diagnostic attributes
//! note: The program prints, then panics on the index; CARDIAC's output should end where the native one does.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[allow(unconditional_panic)]
pub fn main() {
    let a = inp();
    let t = [a, a + 1, a + 2];
    out(t[1]);
    out(t[3]);
}
