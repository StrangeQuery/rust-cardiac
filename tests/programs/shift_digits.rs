#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A digit or two from the middle of a value, by two shifts made one: `m / 10 % 10` is `SFT 23`, `m / 100 % 10`
/// `SFT 13`, `m / 10 % 100` `SFT 12`, and `m % 100 / 10` `SFT 23` again; negative values' digits negative, as Rust's
/// / and % give them.
pub fn main() {
    let vs: [i16; 9] = [0, 7, -7, 45, -45, 123, -123, 999, -999];
    let mut i = 0;
    while i < 9 {
        let m = vs[i];
        out(m / 10 % 10);
        out(m / 100 % 10);
        out(m / 10 % 100);
        out(m % 100 / 10);
        i += 1;
    }
}
