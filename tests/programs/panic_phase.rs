#![no_std]

/// The teletype. The CARDIAC backend turns calls to this into OUT.
#[inline(never)]
pub fn out(_v: i16) {}

/// The end of a phase: the next phase's code loads over this one's.
#[inline(never)]
pub fn overlay() {}

/// A phase that starts with a panic: its jump to the panic routine, which comes next, stays, so the panic map
/// finds the check in the phase it is in.
#[allow(unconditional_panic)]
pub fn main() {
    out(6);
    overlay();
    let mut a: [i16; 5] = [5, 3, 100, 100, 99];
    a[101] = 6;
    out(a[0]);
}
