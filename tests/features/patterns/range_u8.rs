//! feature: Ranges on a `u8` (`b'0'..=b'9'`)
//! section: Range patterns
//! input: 1 2 3 4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const CODES: [u8; 5] = [0, b'0', b'9', b':', b'A'];
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let c = CODES[v as usize];
        let r = match c {
            b'0'..=b'9' => 1,
            b'A'..=b'Z' => 2,
            _ => 0,
        };
        out(r);
    }
}
