#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

fn divide(n: i16, d: i16) -> (i16, i16) {
    let mut rest = n;
    let mut quotient = 0;
    while rest >= d {
        rest -= d;
        quotient += 1;
    }
    (quotient, rest)
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

/// Two sine waves added up a row at a time, (a + 2 m) mod 16 and (y / 3 + 32 - n) mod 16, the total turned round 16
/// colours. Its index into the sine table is a sum carried straight into a subtraction: the range pass must see all of
/// it to prove the bounds check. Must not grow past 89 cells.
pub fn main() {
    let sine = SINE;
    let y = inp();
    let clock_m = inp();
    let clock_n = inp();
    unsafe {
        assume(y >= 0);
        assume(clock_m >= 0);
        assume(clock_n < 16);
    }
    // the wave rolling up: S[(y / 3 + 32 - n) mod 16], y / 3 by repeated subtraction
    let (third_y, _) = divide(y, 3);
    let step = remainder(third_y + 32 - clock_n, 16);
    let rolling = sine[step as usize];
    for _ in 0..96 {
        // 96 columns
        let third = inp(); // x / 3
        let rings = inp(); // the ring machine's sum
        unsafe { assume(third >= 0) };
        // the wave sliding across: S[(x / 3 + 2 m) mod 16]
        let step = remainder(third + clock_m + clock_m, 16);
        let mut value = rings + sine[step as usize] + rolling;
        // the colour: value / 16 + m, turned once round the 16 colours at most
        let mut colour = clock_m;
        while value >= 16 {
            value -= 16;
            colour += 1;
        }
        if colour >= 16 {
            colour -= 16;
        }
        out(colour);
    }
}
