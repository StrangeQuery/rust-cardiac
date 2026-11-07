//! feature: A `#[repr(i16)]` C-like enum
//! section: Enumerations
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(i16)]
#[derive(Clone, Copy)]
enum Op { Add = 1, Sub = -1 }
#[inline(never)]
fn apply(op: Op, a: i16, b: i16) -> i16 { match op { Op::Add => a + b, Op::Sub => a - b } }
pub fn main() {
    let op = if inp() == 0 { Op::Add } else { Op::Sub };
    out(apply(op, 10, 3));
    out(op as i16);
    let op2 = if inp() == 0 { Op::Add } else { Op::Sub };
    out(apply(op2, 10, 3));
}
