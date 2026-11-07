//! feature: An `extern "C"` block declaring a function
//! section: External blocks
//! disposition: never
//! note: There is nothing on the other side of an FFI call on CARDIAC; the card reader and the printer are the whole outside world. The call never runs here, but natively it needs a definition to link.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
extern "C" { fn host_add(a: i16, b: i16) -> i16; }
#[no_mangle]
pub extern "C" fn host_add_impl(a: i16, b: i16) -> i16 { a + b }
pub fn main() {
    let v = inp();
    if v == 0 { out(unsafe { host_add(v, 1) }); }
    out(host_add_impl(v, 1));
}
