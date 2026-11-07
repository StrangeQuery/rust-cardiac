//! feature: Match ergonomics: `&Option<i16>` matched with `Some(x)`
//! section: Binding modes
//! input: 5 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn show(o: &Option<i16>) -> i16 {
    match o {
        Some(x) => *x,
        None => -100,
    }
}
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let o = if v > 0 { Some(v) } else { None };
        out(show(&o));
    }
}
