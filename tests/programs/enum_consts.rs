#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Constants of an enum with data: `const Result::<i16, i16>::Err(-9_i16)`, from ok_or at opt-level 2, assigned
/// and passed as an argument.
#[inline(never)]
fn pos(v: i16) -> Option<i16> {
    if v >= 0 { Some(v + 1) } else { None }
}

#[inline(never)]
fn show(r: Result<i16, i16>) {
    match r { Ok(x) => out(x), Err(e) => out(e - 100) }
}

pub fn main() {
    let mut v = -2;
    while v < 3 {
        show(pos(v).ok_or(-9));
        show(Err(4));
        v += 1;
    }
}
