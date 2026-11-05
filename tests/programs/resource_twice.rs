#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
pub fn load_resource<const N: usize>(_name: &str) -> [i16; N] {
    [0; N]
}

/// Rejected: one resource, one slot, loaded once.
pub fn main() {
    let a: [i16; 2] = load_resource("key");
    let b: [i16; 2] = load_resource("key");
    out(a[0] + b[1]);
}
