#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn digit(v: i16, k: i16) -> i16 { if k == 0 { v / 100 } else if k == 1 { v / 10 % 10 } else { v % 10 } }

/// digit() at a constant place is the shift itself, `SFT (k+1)3`, each place, a negative value's digits
/// negative as Rust's are; and a digit of a value known not negative is 0 to 9, so it indexes a table with no
/// check.
pub fn main() {
    let names: [i16; 10] = [100, 101, 102, 103, 104, 105, 106, 107, 108, 109];
    let vs: [i16; 6] = [-400, -126, 148, 422, 0, 999];
    let mut i = 0;
    while i < 6 {
        let v = vs[i];
        out(digit(v, 0));
        out(digit(v, 1));
        out(digit(v, 2));
        if v >= 0 {
            out(names[digit(v, 1) as usize]);
        }
        i += 1;
    }
}
