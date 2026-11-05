#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Expression form: a helper that picks one of its arguments, and if/else used as a value.
fn clamp(v: i16, lo: i16, hi: i16) -> i16 { if v < lo { lo } else if v > hi { hi } else { v } }

pub fn main() {
    let mut seed = 7;
    let mut k = -12;
    loop {
        seed += 37;
        if seed >= 90 { seed -= 90; }
        out(clamp(seed - 20, 0, 50));
        let m = if seed > 45 { seed - 45 } else { 45 - seed };
        out(m);
        k += 1;
        if k >= 0 { break; }
    }
}
