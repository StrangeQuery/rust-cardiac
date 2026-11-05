//! The arithmetic the ray machines share, which CARDIAC has no instructions for: a square root, a remainder and a
//! division, each by repeated subtraction. They are small, so rustc inlines them into each machine.
#![allow(dead_code)]

/// The integer square root of n (0 for n below 1): the odd numbers 1, 3, 5, ...
/// taken away while they fit, counted, since their sums are the squares.
pub fn isqrt(mut n: i16) -> i16 {
    let mut root = 0;
    let mut odd = 1;
    while n >= odd {
        n -= odd;
        odd += 2;
        root += 1;
    }
    root
}

/// v mod m, for v of 0 or more: m taken away while it fits.
pub fn remainder(v: i16, m: i16) -> i16 {
    let mut left = v;
    while left >= m {
        left -= m;
    }
    left
}

/// n / d and n mod d, for n of 0 or more and d of 1 or more: d taken away while it fits, counted.
pub fn divide(n: i16, d: i16) -> (i16, i16) {
    let mut rest = n;
    let mut quotient = 0;
    while rest >= d {
        rest -= d;
        quotient += 1;
    }
    (quotient, rest)
}

/// The size of v.
pub fn abs(v: i16) -> i16 {
    if v < 0 {
        -v
    } else {
        v
    }
}
