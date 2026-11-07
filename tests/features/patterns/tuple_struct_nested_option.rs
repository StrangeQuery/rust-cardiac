//! feature: Nested tuple-struct patterns (`Some(Ok(x))`)
//! section: Tuple struct patterns
//! input: 5 -5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn get(v: i16) -> Option<Result<i16, i16>> {
    if v > 0 { Some(Ok(v)) } else if v < 0 { Some(Err(-v)) } else { None }
}
pub fn main() {
    for _ in 0..3 {
        let v = inp();
        let r = match get(v) {
            Some(Ok(x)) => x,
            Some(Err(e)) => -e * 10,
            None => 0,
        };
        out(r);
    }
}
