#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// saturating_sub of u8, a call at opt-level 1 and s and an intrinsic rustc calls at 2: a - b, or 0.
pub fn main() {
    let us: [u8; 4] = [0, 3, 9, 200];
    let mut i = 0usize;
    while i < 4 {
        let u = us[i];
        out(u.saturating_sub(5) as i16);
        out(5u8.saturating_sub(u) as i16);
        i += 1;
    }
}
