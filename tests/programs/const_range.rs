#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// A slice, in a function main never calls: not what stops the build.
#[inline(never)]
pub fn first(a: &[i16; 2]) -> i16 {
    let s: &[i16] = a;
    s[0]
}

/// A constant no card holds, 2000: rejected, with the line it is on. (Not `x / 2000`, which is 0 for every card,
/// and needs no cell.)
pub fn main() {
    let x = inp();
    out(x + 2000);
}
