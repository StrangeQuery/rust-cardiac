//! feature: Or-pattern in `let` (`let (Ok(x) | Err(x)) = r;`)
//! section: Or-patterns
//! input: 5 -5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn get(v: i16) -> Result<i16, i16> { if v > 0 { Ok(v) } else { Err(-v * 2) } }
pub fn main() {
    for _ in 0..2 {
        let (Ok(x) | Err(x)) = get(inp());
        out(x);
    }
}
