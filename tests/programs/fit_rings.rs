#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

fn remainder(v: i16, m: i16) -> i16 {
    let mut left = v;
    while left >= m {
        left -= m;
    }
    left
}

unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

const SINE: [i16; 16] = [32, 44, 54, 61, 63, 61, 54, 44, 32, 19, 9, 2, 0, 2, 9, 19];

/// Rings and a slanting wave from a sine table, (d - m) mod 16 and (e + n) mod 16. Its index is `d + 32` carried
/// straight into a subtraction, as in fit_mix. Must not grow past 61 cells.
pub fn main() {
    let sine = SINE;
    let _y = inp();
    let clock_m = inp();
    let clock_n = inp();
    unsafe {
        assume(clock_m < 16);
        assume(clock_n >= 0);
    }
    for _ in 0..96 {
        // 96 columns
        let distance = inp(); // from the middle, in two-pixel steps
        let slant = inp(); // the place along the slant
        unsafe {
            assume(distance >= 0);
            assume(slant >= 0);
        }
        let ring_step = remainder(distance + 32 - clock_m, 16); // rings spreading out: (d - m) mod 16
        let slant_step = remainder(slant + clock_n, 16); // the slanting wave: (e + n) mod 16
        out(sine[ring_step as usize] + sine[slant_step as usize]);
    }
}
