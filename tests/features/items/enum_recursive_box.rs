//! feature: A recursive enum through `Box`
//! section: Enumerations
//! disposition: never
//! note: `Box` needs a heap; this row uses a `&'static` link instead, the nearest a `no_std` program gets without an allocator.
//! input:
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    // `Box` is in `alloc`, not `core`: without a heap this cannot even be written in a `no_std` crate here.
    enum List { Nil, Cons(i16, &'static List) }
    static END: List = List::Nil;
    let l = List::Cons(inp(), &END);
    match l { List::Cons(v, _) => out(v), List::Nil => out(0) }
}
