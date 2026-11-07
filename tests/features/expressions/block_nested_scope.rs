//! feature: Nested blocks shadowing a name
//! section: Block expressions
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let r = {
        let a = a + 1;
        {
            let a = a * 2;
            out(a);
        }
        a
    };
    out(r);
    out(a);
}
