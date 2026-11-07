//! feature: An array of tuples `[(i16, i16); 3]`
//! section: Array types
//! disposition: demand
//! note: The compiler could keep one array per tuple field, as it does for an array of structs.
//! input: 1 2 3 4 5 6 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t = [(0i16, 0i16); 3];
    let mut k = 0;
    while k < 3 {
        t[k] = (inp(), inp());
        k += 1;
    }
    let i = inp() as usize;
    out(t[i].0);
    out(t[i].1);
}
