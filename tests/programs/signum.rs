#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// abs's kin, which rustc calls at opt-level 1 and s: signum, is_negative, is_positive, wrapping_neg,
/// wrapping_abs; and abs_diff of u8. (At opt-level 2 rustc makes signum a three-way compare, and abs_diff of u8 a
/// subtraction in i32, which are refused.)
pub fn main() {
    let xs: [i16; 5] = [-999, -7, 0, 3, 200];
    let us: [u8; 5] = [0, 3, 9, 10, 250];
    let mut i = 0usize;
    while i < 5 {
        let x = xs[i];
        out(x.signum());
        out(x.is_negative() as i16);
        out(x.is_positive() as i16);
        out(x.wrapping_neg());
        out(x.wrapping_abs());
        out(us[i].abs_diff(9) as i16);
        i += 1;
    }
}
