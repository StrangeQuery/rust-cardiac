#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// An enum with data behind a `&mut`: a `&mut` of each of its cells.
#[inline(never)]
fn bump(o: &mut Option<i16>) {
    match o {
        Some(v) if *v >= 12 => *o = None,
        Some(v) => *v += 1,
        None => *o = Some(10),
    }
}

pub fn main() {
    let mut o: Option<i16> = None;
    let mut i = 0;
    while i < 5 {
        bump(&mut o);
        out(o.unwrap_or(-1));
        i += 1;
    }
}
