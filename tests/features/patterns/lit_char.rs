//! feature: `char` literals (`'a' =>`)
//! section: Literal patterns
//! disposition: easy
//! note: A `char` can hold up to 1114111, far past a card, but every ASCII and Latin-1 character fits: like an `i16`, it would work where the value fits.
//! input: 97 98 122 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let c = v as u8 as char;
        let r = match c {
            'a' => 1,
            'b' => 2,
            _ => 0,
        };
        out(r);
    }
}
