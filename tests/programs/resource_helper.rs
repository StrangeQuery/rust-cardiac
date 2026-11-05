#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
pub fn load_resource<const N: usize>(_name: &str) -> [i16; N] {
    [0; N]
}

/// Rejected: a load outside main may run more than once.
#[inline(never)]
pub fn first() -> i16 {
    let key: [i16; 2] = load_resource("key");
    key[0]
}

pub fn main() {
    out(first());
}
