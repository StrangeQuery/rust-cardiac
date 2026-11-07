# Unsafety

[The rust-cardiac manual](../manual.md) › Unsafety

## What's different on CARDIAC

- `assume()` matters more than in Rust: every check the compiler can drop saves cells out of 96.
- Everything else on this page works as in Rust.

## assume

`unsafe { assume(cond) }`, from the `cardiac` crate, is Rust's `assert_unchecked`. It lets the compiler leave out a
check it can then prove. Break the assumption and the program reads whatever is there. With debug assertions on
(`debug-assertions` in the profile), the assumption is checked on CARDIAC too. Natively it always is.

`assume()` also helps the range pass: an `Option<i16>` whose payload is assumed never negative needs no tag cell.
See [Types](types.md#enums).

## Unchecked operations

`unchecked_add` and its kin, `core::hint::assert_unchecked` and `unreachable_unchecked` work. With debug assertions
on, their preconditions are checked, and a violated one aborts: see [Panic](panic.md#aborts).

## Raw pointers, unions and mutable statics

Raw pointers work, with `p.add(i)`. A `union` works when it is read through the field it was written by. `static
mut` works.

Feature tests: [types](../../tests/features/types) and [items](../../tests/features/items).
