#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

#[inline(never)]
pub fn bump(x: &mut u8) {
    *x += 1;
}

#[inline(never)]
pub fn swap(a: &mut u8, b: &mut u8) {
    let t = *a;
    *a = *b;
    *b = t;
}

/// Mutable references are cell addresses; the borrow checker proved they never alias.
pub fn main() {
    let mut n: u8 = 1;
    let mut m: u8 = 50;
    bump(&mut n);
    bump(&mut n);
    out(n);
    swap(&mut n, &mut m);
    out(n);
    out(m);
}
