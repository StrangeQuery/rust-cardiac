//! feature: `Result::map`
//! section: Result
//! input: 5 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn ok_if(a: i16) -> Result<i16, i16> {
    if a > 0 { Ok(a) } else { Err(-a) }
}
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let r = ok_if(a);
        out(r.map(|x| x + 1).unwrap_or(0));
    }
}
