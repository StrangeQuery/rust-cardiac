//! feature: `a.next_power_of_two()` on `u8`
//! section: Powers and roots
//! disposition: demand
//! note: rustc counts leading zeros (in `u32`); on a card it would be a loop of doublings.
//! input: 0 1 2 3 4 5 -1
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
        out(a.next_power_of_two() as i16);
    }
}
