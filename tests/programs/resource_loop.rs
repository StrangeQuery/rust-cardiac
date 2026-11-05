#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
pub fn load_resource<const N: usize>(_name: &str) -> [i16; N] {
    [0; N]
}

/// Rejected: a load in a loop runs more than once, and the loader fills its slot only once.
pub fn main() {
    let mut i: i16 = 0;
    while i < 3 {
        let mut key: [i16; 2] = load_resource("key");
        key[0] += i;
        out(key[0]);
        i += 1;
    }
}
