#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn parse(v: i16) -> Result<i16, i16> {
    if v < 3 {
        Ok(v)
    } else {
        Err(-v)
    }
}

/// Result's `map` with a closure that captures nothing: Ok's value through the closure, Err as it is.
pub fn main() {
    let mut v = 0;
    while v < 6 {
        match parse(v).map(|x| x + x + 1) {
            Ok(x) => out(x),
            Err(e) => out(e - 100),
        }
        v += 1;
    }
}
