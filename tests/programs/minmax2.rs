#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// min, max and clamp of u8 and usize, which rustc calls at every opt-level.
pub fn main() {
    let us: [u8; 5] = [0, 3, 9, 200, 254];
    let mut i = 0usize;
    while i < 5 {
        let u = us[i];
        out(u.min(9) as i16);
        out(u.max(5) as i16);
        out(u.clamp(3, 200) as i16);
        out(i.max(3) as i16);
        out(i.clamp(1, 3) as i16);
        i += 1;
    }
}
