#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Result's methods, which rustc calls at opt-level 1: ok, err and unwrap_or.
#[inline(never)]
fn parse(v: i16) -> Result<i16, i16> {
    if v < 0 {
        Err(v - 5)
    } else {
        Ok(v)
    }
}

pub fn main() {
    let mut v = -2;
    while v < 2 {
        let r = parse(v);
        out(r.err().unwrap_or(0));
        out(r.ok().unwrap_or(-1));
        v += 1;
    }
}
