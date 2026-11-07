//! feature: `#[cfg(test)]` code and `cfg!(test)`
//! section: Testing attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[cfg(test)]
fn value() -> i16 { 1 }
#[cfg(not(test))]
fn value() -> i16 { 2 }
pub fn main() {
    let a = inp();
    out(a + value());
    out(if cfg!(test) { 1 } else { 0 });
}
