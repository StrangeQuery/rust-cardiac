//! feature: Byte literals (`b'A' =>`) on a `u8`, read from a `u8` table
//! section: Literal patterns
//! input: 1 2 3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const CODES: [u8; 4] = [0, b'A', b'B', b'Z'];
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let c = CODES[v as usize];
        let r = match c {
            b'A' => 1,
            b'B' => 2,
            _ => 0,
        };
        out(r);
    }
}
