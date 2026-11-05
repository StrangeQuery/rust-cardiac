#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
pub fn load_resource<const N: usize>(_name: &str) -> [i16; N] {
    [0; N]
}

/// A resource is loaded once, in main, and is the program's own array after: it may write to it, and a copy of it
/// is cells of its own.
pub fn main() {
    let mut key: [i16; 3] = load_resource("key");
    out(key[1]);
    let copy = key;
    key[1] = 7;
    key[2] = key[1] + 1;
    out(key[1]);
    out(copy[1]);
    out(key[2] + copy[2]);
}
