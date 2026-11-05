#[inline(never)]
pub fn out(_v: i16) {}

/// A generic function from another crate: rustc gives it no MIR this compiler can use, so it is an error.
pub fn main() {
    out(kit::first([7, 8, 9]));
}
