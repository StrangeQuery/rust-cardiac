//! feature: A literal inside a variant (`Some(0)`)
//! section: Tuple struct patterns
//! input: 0 3 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn get(v: i16) -> Option<i16> { if v >= 0 { Some(v) } else { None } }
pub fn main() {
    for _ in 0..3 {
        let v = inp();
        let r = match get(v) {
            Some(0) => 100,
            Some(x) => x,
            None => -1,
        };
        out(r);
    }
}
