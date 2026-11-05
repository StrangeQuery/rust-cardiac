#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 5] = [4, 8, 15, 16, 23];

/// With debug assertions on, rustc keeps core's checks of unsafe preconditions: `a.unchecked_sub(b)` first
/// calls `unchecked_sub::precondition_check(a, b)`, a function of core's the compiler has no MIR for. Each is an
/// assert of what it checks, and panics as Rust does (natively, an abort) when it fails: `sub(3, 5)` at the end.
#[inline(never)]
fn add(a: usize, b: usize) -> usize {
    unsafe { a.unchecked_add(b) }
}

#[inline(never)]
fn sub(a: usize, b: usize) -> usize {
    unsafe { a.unchecked_sub(b) }
}

#[inline(never)]
fn sum(a: i16, b: i16) -> i16 {
    unsafe { a.unchecked_add(b) }
}

#[inline(never)]
fn diff(a: i16, b: i16) -> i16 {
    unsafe { a.unchecked_sub(b) }
}

pub fn main() {
    out(add(400, 99) as i16);
    out(sub(400, 99) as i16);
    out(sum(-7, 900));
    out(diff(-400, 99));
    out(sub(3, 5) as i16);
}
