//! feature: `#[derive(PartialOrd)]` on a C-like enum: `<` by declaration order
//! section: Derive
//! disposition: easy
//! input: 0 1 2 1 1 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(PartialEq, PartialOrd, Clone, Copy)]
enum Lvl { Low, Mid, High }
fn lvl(v: i16) -> Lvl { if v == 0 { Lvl::Low } else if v == 1 { Lvl::Mid } else { Lvl::High } }
pub fn main() {
    loop {
        let a = inp();
        if a < 0 { break; }
        let b = inp();
        out(if lvl(a) < lvl(b) { 1 } else { 0 });
    }
}
