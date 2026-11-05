#![no_std]
#![allow(unconditional_panic)]
#[inline(never)]
pub fn out(_v: i16) {}

/// The panic is main's last line, after a loop: its JMP rt_panic is dropped as the routine comes next, so main runs
/// off its end into the routine, which nothing names. It stays, with its label, for the panic map to find it by: a
/// run that reached it unlabelled halted as if it had ended, exit 0.
pub fn main() {
    for i in 0..0 {
        out(i);
    }
    out(0);
    out(0 / 0);
}
