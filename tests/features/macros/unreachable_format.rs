//! feature: `unreachable!("{}", x)` with a formatted message
//! section: Panics
//! disposition: demand
//! note: The check could keep the message's literal text and leave out the values; formatting the values themselves is never.
//! input: 1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let v = match a {
            1 => 10,
            _ => unreachable!("card {}", a),
        };
        out(v);
    }
}
