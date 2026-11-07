//! feature: `let Some(x) = o else { ... };`
//! section: Destructuring
//! input: 5 -1 7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pos(v: i16) -> Option<i16> { if v > 0 { Some(v) } else { None } }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let Some(x) = pos(v) else {
            out(-1);
            continue;
        };
        out(x * 2);
    }
}
