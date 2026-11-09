//! feature: An array of arrays `[[i16; 2]; 2]`
//! section: Array types
//! disposition: demand
//! note: It is one array, laid out by tens where the program fits: `t[r][c]` is `t[10 * r + c]`, the row one shift.
//! input: 1 2 3 4 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t = [[0i16; 2]; 2];
    let mut r = 0;
    while r < 2 {
        let mut c = 0;
        while c < 2 {
            t[r][c] = inp();
            c += 1;
        }
        r += 1;
    }
    let r = inp() as usize;
    let c = inp() as usize;
    out(t[r][c]);
}
