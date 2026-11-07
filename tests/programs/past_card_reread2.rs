#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

struct Box1 {
    v: i16,
}

/// Without overflow checks a product past a card is what its cell keeps (1200 is kept as 200): read again after it
/// is stored, by a test, a second test, a sum, and from a field, it is what was printed
#[inline(never)]
fn plain(a: i16) {
    let p = a * 40;
    out(p);
    if p > 500 {
        out(1);
    }
    if p < 300 {
        out(2);
    }
    out(p + 1);
}

#[inline(never)]
fn field(a: i16) {
    let mut b = Box1 { v: 0 };
    b.v = a * 40;
    out(b.v);
    out(if b.v > 500 { 1 } else { 0 });
}

pub fn main() {
    plain(30);
    field(30);
}
