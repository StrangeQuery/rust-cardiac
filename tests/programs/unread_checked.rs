#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Checked ops whose value nothing reads. A sum or difference of two cards can't pass an i16, so it can't
/// panic natively either: it goes, check and all, even at 999 + 1, which is past a card. A product of two cards can
/// (999 * 999): it stays for its panic, which 200 * 300 gives both ways.
#[inline(never)]
fn unread(a: i16, b: i16) {
    let _sum = a + 1;
    let _diff = b - a;
    out(a);
}

#[inline(never)]
fn unread_product(a: i16, b: i16) {
    let _product = a * b;
    out(b);
}

pub fn main() {
    unread(999, -999);
    unread(-999, 999);
    unread(5, 7);
    unread_product(5, -7);
    unread_product(200, 300);
}
