//! Remainders by 2 and 5, by the last digit: of a value of any sign (Rust's takes the dividend's), `rem_euclid`,
//! and of a usize.
use cardiac::*;

fn main() {
    let x = inp();
    out(x % 2);
    out(x % -5);
    out(x.rem_euclid(5));
    if x >= 0 {
        out((x as usize % 2) as i16);
    }
}
