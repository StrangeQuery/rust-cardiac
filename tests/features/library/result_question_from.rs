//! feature: `?` converting the error with `From`
//! section: Result
//! note: A wrong answer at opt-level 1: the error comes out unconverted, as if `From::from` weren't called (-3 where Rust prints -30).
//! input: -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Small(i16);
struct Big(i16);
impl From<Small> for Big {
    fn from(s: Small) -> Big {
        Big(s.0 * 10)
    }
}
#[inline(never)]
fn check(a: i16) -> Result<i16, Small> {
    if a > 0 { Ok(a) } else { Err(Small(-a)) }
}
#[inline(never)]
fn run(a: i16) -> Result<i16, Big> {
    let v = check(a)?;
    Ok(v + 1)
}
pub fn main() {
    let a = inp();
    out(match run(a) { Ok(v) => v, Err(Big(e)) => -e });
}
