//! feature: A function returning a reference (`fn id(a: &i16) -> &i16`)
//! section: Generic parameters
//! note: A wrong answer: the reference that comes back reads the wrong cell (609 for 8), at every setting.
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn id(a: &i16) -> &i16 { a }
pub fn main() {
    let a = inp();
    out(*id(&a));
}
