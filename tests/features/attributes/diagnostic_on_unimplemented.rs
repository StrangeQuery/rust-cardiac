//! feature: `#[diagnostic::on_unimplemented]` and `#[diagnostic::do_not_recommend]`
//! section: Diagnostic attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[diagnostic::on_unimplemented(message = "not a card")]
trait Card { fn v(&self) -> i16; }
impl Card for i16 { fn v(&self) -> i16 { *self + 1 } }
pub fn main() {
    out(inp().v());
}
