//! feature: `t[i] += a` on an array element
//! section: Assignment and compound assignment
//! input: 0 5 2 -3 0 4 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t: [i16; 3] = [10, 20, 30];
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        t[i as usize] += inp();
    }
    out(t[0]);
    out(t[1]);
    out(t[2]);
}
