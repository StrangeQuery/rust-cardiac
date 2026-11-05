#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Expression form: tuples taken apart into variables (a swap, a rotation), and lets read once.
pub fn main() {
    let mut seed = 5;
    let mut x = 0;
    let mut y = 1;
    let mut z = 2;
    let mut total = 0;
    let mut k = -10;
    loop {
        seed += 23;
        if seed >= 50 { seed -= 50; }
        let (a, b) = if seed < 20 { (y, x) } else { (x + 1, y) };
        x = a;
        y = b;
        let (p, q, r) = if seed < 35 { (y + 2, z, x) } else { (z, x, y) };
        x = p;
        y = q;
        z = r;
        out(x);
        out(y);
        out(z);
        let d = seed + 3;
        let e = d + d;
        total += e;
        if total >= 300 { total -= 300; }
        out(total);
        k += 1;
        if k >= 0 { break; }
    }
}
