//! feature: `core::write!` and `format_args!` into a `core::fmt::Write`
//! section: Formatting macros
//! disposition: never
//! note: String formatting: the `fmt` machinery is dynamic dispatch over trait objects and digit loops in `u32`.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::fmt::Write;
struct Count(i16);
impl Write for Count {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0 += s.len() as i16;
        Ok(())
    }
}
pub fn main() {
    let a = inp();
    let mut c = Count(0);
    let _ = write!(c, "{}", a);
    out(c.0);
}
