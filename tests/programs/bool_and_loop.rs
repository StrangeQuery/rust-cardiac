#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
/// A value rustc can't see through, as a card read would be.
#[inline(never)]
pub fn card(v: i16) -> i16 {
    v
}

/// A bool from `a && b` tested at once and again later, in a loop, as a larger program first had it: a
/// hit on an end wall punches the distance past it, not -1.
pub fn main() {
    let mut start = card(-75);
    let near = card(0);
    let far = card(40);
    for _ in 0..3 {
        let quarters = card(49);
        let past_near = quarters - near;
        let past_far = quarters - far;
        if start < 880 && start + 104 > 0 {
            let end_wall = start > 0 && past_near >= 0;
            if (end_wall && past_near < 120) || (past_near < 0 && past_far >= 0) {
                out(start);
                out(if end_wall { past_near } else { -1 });
            }
        }
        start += 60;
    }
}
