#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

static mut K: i16 = 4;
static mut T: [i16; 3] = [0, 0, 0];
static U: [i16; 3] = [-7, 8, -9];

/// A function that writes a static, called from a loop: its caller sees the new value.
#[inline(never)]
fn bump(n: i16) {
    unsafe { K += n };
}

/// A static read and written straight through its cells, as a local is: a scalar, an array at a constant and at a
/// variable index, and a table that is never written; and the scalar changed by a call in between.
pub fn main() {
    let vs: [i16; 3] = [3, -5, 10];
    let mut i = 0;
    while i < 3 {
        let x = vs[i];
        out(x + unsafe { K });
        bump(x);
        out(unsafe { K });
        unsafe { T[i] = x + U[i] };
        unsafe { T[0] += 1 };
        i += 1;
    }
    let mut j = 0;
    while j < 3 {
        out(unsafe { T[j] });
        j += 1;
    }
}
