#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn last(t: &[[i16; 3]; 2], r: usize) -> i16 {
    t[r][2]
}

#[inline(never)]
fn bump(t: &mut [[i16; 3]; 2], r: usize) {
    t[r][1] += 100;
}

/// A literal grid by tens, passed by & and by &mut and indexed there; then read past its last row, which panics.
pub fn main() {
    let mut t = [[1i16, 2, 3], [4, 5, 6]];
    bump(&mut t, 1);
    out(t[1][1] + last(&t, 0));
    out(last(&t, 2));
}
