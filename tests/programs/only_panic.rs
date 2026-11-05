#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// main is only a panic: no instruction comes before the panic routine, and the program's start runs into it. Its
/// label stays for the panic map, and the JMP stays to say why: with neither, the run exited 0.
pub fn main() {
    panic!();
}
