//! feature: `Result::and_then`
//! section: Result
//! disposition: easy
//! note: At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out.
//! input: 5 50 -3
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
    for _ in 0..3 {
        let a = inp();
        let r = ok_if(a);
        let s = r.and_then(|x| if x < 10 { Ok(x * 2) } else { Err(99) });
        out(match s { Ok(v) => v, Err(e) => -e });
    }
}
