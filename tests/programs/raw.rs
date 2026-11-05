#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Raw pointers allow arithmetic; dereferencing them needs `unsafe`.
pub fn main() {
    let mut a: [u8; 4] = [10, 20, 30, 40];
    let p: *mut u8 = &raw mut a[0];
    unsafe {
        *p.add(2) = 99;
        out(*p.add(3));
        out(*p.add(1));
    }
    out(a[2]);
}
