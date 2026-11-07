//! feature: `let Some(v) = e else { ... };` (let-else)
//! section: Let-else
//! input: 7 -3 4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn positive(x: i16) -> Option<i16> {
    if x > 0 { Some(x) } else { None }
}
pub fn main() {
    loop {
        let a = inp();
        if a == 0 {
            break;
        }
        let Some(v) = positive(a) else {
            out(-1);
            continue;
        };
        out(v + 1);
    }
}
