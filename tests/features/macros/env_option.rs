//! feature: `option_env!` of an unset variable
//! section: Other core macros
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let v = match option_env!("CARDIAC_SUPPORT_UNSET_VARIABLE") {
        Some(_) => 1,
        None => 0,
    };
    out(a + v);
}
