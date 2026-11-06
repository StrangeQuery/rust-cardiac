#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `?` converting the error with From, into a Result whose error is another type. At opt-level 2 rustc inlines
/// `from_residual` and the conversion; at 1 it calls core's, which can't be written out without the conversion, so
/// it is refused.
struct Small(i16);
struct Big(i16);

impl From<Small> for Big {
    fn from(s: Small) -> Big {
        Big(s.0 * 10)
    }
}

#[inline(never)]
fn check(x: i16) -> Result<i16, Small> {
    if x >= 0 {
        Ok(x)
    } else {
        Err(Small(x))
    }
}

#[inline(never)]
fn twice(x: i16) -> Result<i16, Big> {
    let v = check(x)?;
    Ok(v + v)
}

#[inline(never)]
fn show(x: i16) {
    match twice(x) {
        Ok(v) => out(v),
        Err(Big(e)) => out(e),
    }
}

pub fn main() {
    show(-3);
    show(4);
}
