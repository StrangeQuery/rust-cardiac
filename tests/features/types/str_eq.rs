//! feature: `==` of two string literals chosen by inputs
//! section: Textual types
//! disposition: never
//! note: A string compare is a byte loop over every literal it might meet, each a cell a byte: two short literals compared come to about 124 cells, more than CARDIAC has.
//! input: 1 1 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pick(x: i16) -> &'static str {
    if x > 0 { "yes" } else { "no" }
}
pub fn main() {
    let mut k = 0;
    while k < 2 {
        let a = pick(inp());
        let b = pick(inp());
        out((a == b) as i16);
        k += 1;
    }
}
