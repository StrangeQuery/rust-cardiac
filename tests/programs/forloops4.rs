#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn count(start: u8, mut end: u8) -> u8 {
    let mut last = 0;
    for b in start..end {
        end -= 1;
        last = b;
    }
    out(end as i16);
    last
}

pub fn main() {
    out(count(2, 7) as i16);
    let mut found = -1;
    for mut i in 1..20 {
        i *= 2;
        if i > 9 {
            found = i;
            break;
        }
    }
    out(found);
}
