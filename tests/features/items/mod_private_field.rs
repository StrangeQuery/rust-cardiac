//! feature: A struct with a private field, built by a module's `pub fn new`
//! section: Modules
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod counter {
    pub struct C { n: i16 }
    impl C {
        pub fn new(n: i16) -> C { C { n } }
        pub fn get(&self) -> i16 { self.n }
        pub fn bump(&mut self) { self.n += 1; }
    }
}
pub fn main() {
    let mut c = counter::C::new(inp());
    c.bump();
    c.bump();
    out(c.get());
}
