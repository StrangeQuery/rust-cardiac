#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `opt == Some(c)` with the niche layout, as a larger program's stamp machine reads ahead: the payload is compared
/// with c first, and c's own test (0 or more, as a counter is) is one the range pass decides. At opt-level 1
/// rustc calls Option's `eq`, with `None` a promoted constant.
#[inline(never)]
fn find(t: &[i16; 5], v: i16) -> Option<i16> {
    let mut i = 0;
    while i < 5 {
        if t[i as usize] == v {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn main() {
    let t = [7, 3, 9, 3, 5];
    let mut v = 2;
    while v < 11 {
        let found = find(&t, v);
        let mut c = 0;
        while c < 5 {
            if found == Some(c) {
                out(c);
            }
            c += 1;
        }
        if found == None {
            out(-1);
        }
        v += 1;
    }
}
