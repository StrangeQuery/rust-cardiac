//! feature: Match ergonomics through `&mut`: `if let Some(x) = o` with `o: &mut Option<i16>`
//! section: Binding modes
//! input: 5 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn bump(o: &mut Option<i16>) {
    if let Some(x) = o {
        *x += 1;
    }
}
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let mut o = if v > 0 { Some(v) } else { None };
        bump(&mut o);
        out(o.unwrap_or(-100));
    }
}
