//! feature: a `struct` with an `impl` declared inside a function
//! section: Item declarations
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    struct Pair {
        a: i16,
        b: i16,
    }
    impl Pair {
        fn sum(&self) -> i16 {
            self.a + self.b
        }
    }
    let p = Pair { a: inp(), b: inp() };
    out(p.sum());
}
