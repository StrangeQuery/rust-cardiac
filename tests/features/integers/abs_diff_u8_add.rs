//! feature: two `u8::abs_diff`s in one function, added with `+` (a panic past 255 with overflow checks on)
//! section: Sign and absolute value
//! input: 0 100 50 60 0 200 255 0 10 20 30 30 255 0 0 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let x = inp();
        if x < 0 {
            break;
        }
        let a = x as u8;
        let b = inp() as u8;
        let c = inp() as u8;
        let d = inp() as u8;
        let s = a.abs_diff(b) + c.abs_diff(d);
        out(s as i16);
    }
}
