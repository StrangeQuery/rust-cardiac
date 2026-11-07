//! feature: `use Enum::*` to bring variants into scope
//! section: Use declarations
//! input: 1 2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Dir { Left, Right, Stay }
use Dir::*;
fn pick(v: i16) -> Dir { if v == 1 { Left } else if v == 2 { Right } else { Stay } }
pub fn main() {
    for _ in 0..3 {
        let d = pick(inp());
        out(match d { Left => -1, Right => 1, Stay => 0 });
    }
}
