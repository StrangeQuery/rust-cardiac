#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn find(v: i16) -> Option<i16> {
    if v >= 3 {
        Some(v - 3)
    } else {
        None
    }
}

/// Option's `map` with a closure that captures nothing: at opt-level 1 and s rustc calls `map` with the
/// closure as a zero-sized constant; it is the closure's own function called on Some's value. Two closures, two
/// functions. (Result's: result_map.rs.)
pub fn main() {
    let mut v = 0;
    while v < 6 {
        out(find(v).map(|e| e + 10).unwrap_or(-5));
        out(find(v).map(|e| e + e).unwrap_or(-6));
        v += 1;
    }
}
