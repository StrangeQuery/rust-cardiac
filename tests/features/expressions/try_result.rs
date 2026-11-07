//! feature: `?` on a `Result` in a function returning `Result`
//! section: The question mark operator
//! input: 5 -3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn check(x: i16) -> Result<i16, i16> { if x >= 0 { Ok(x) } else { Err(x) } }
#[inline(never)]
fn twice(x: i16) -> Result<i16, i16> { let v = check(x)?; Ok(v + v) }
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        match twice(a) {
            Ok(v) => out(v),
            Err(e) => out(e * 100),
        }
    }
}
