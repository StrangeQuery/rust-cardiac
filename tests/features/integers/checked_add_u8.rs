//! feature: `a.checked_add(b)` on `u8`
//! section: Checked
//! disposition: easy
//! note: At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`.
//! input: 1 2 5 5 6 1 7 7 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = U[i as usize];
        let b = U[inp() as usize];
        match a.checked_add(b) {
            Some(v) => out(v as i16),
            None => out(-1),
        }
    }
}
