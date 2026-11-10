#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// Digit k of a card (0 the hundreds, 1 the tens, 2 the units): a built-in, which the compiler knows by name.
#[inline(never)]
pub fn digit(v: i16, k: i16) -> i16 {
    if k == 0 {
        v / 100
    } else if k == 1 {
        v / 10 % 10
    } else {
        v % 10
    }
}

/// A square root of a long number (its hundreds and the rest) by taking away 1, 3, 5, ... while they fit, with a
/// borrow from the hundreds; then the parts of what is left and of 20 a + 1. Must not grow past 95 cells.
pub fn main() {
    let count = inp();
    for _ in 0..count {
        let mut xh = inp();
        let mut xl = inp();
        let mut a = 0;
        let mut odd = 1;
        loop {
            let mut nh = xh;
            let mut nl = xl - odd;
            if nl < 0 {
                nl += 999;
                nl += 1;
                nh -= 1;
            }
            if nh < 0 {
                break;
            }
            xh = nh;
            xl = nl;
            a += 1;
            odd += 2;
        }
        // what is left times 100 plus the last two digits
        let lh = digit(xl, 0);
        out(lh * 10 + digit(xl, 1));
        out(digit(xl, 2) * 100 + inp());
        // 20 a + 1: 2 a's hundreds are its thousands
        let a2 = a + a;
        let th = digit(a2, 0);
        out(th);
        out((a2 - th * 100) * 10 + 1);
        let hundreds = digit(a, 0);
        out(hundreds);
        out((a - hundreds * 100) * 10);
    }
}
