#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

struct Lamp {
    on: bool,
    n: i16,
}

/// `!` on a bool array element and on a struct's bool field: each turned over in place, then read.
pub fn main() {
    let mut flags = [false, true, false];
    let mut i = 0;
    while i < 3 { flags[i] = !flags[i]; i += 1; }
    i = 0;
    while i < 3 { if flags[i] { out(i as i16); } i += 1; }
    let mut l = Lamp { on: false, n: 0 };
    while l.n < 5 {
        l.on = !l.on;
        if l.on { out(l.n); }
        l.n += 1;
    }
}
