//! feature: `u8::abs_diff` of neighbours in a loop, summed in an i16
//! section: Sign and absolute value
//! input: 0 255 3 250 7 9 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut prev = inp() as u8;
    let mut total: i16 = 0;
    loop {
        let x = inp();
        if x < 0 {
            break;
        }
        let v = x as u8;
        let d = prev.abs_diff(v);
        out(d as i16);
        total = total + d as i16;
        prev = v;
    }
    out(total);
}
