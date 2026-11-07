# Attributes

[The rust-cardiac manual](../manual.md) › Attributes

## What's different on CARDIAC

- `#[inline(never)]` matters more than in Rust, because every copy of a helper comes out of 96 cells.
- `#[derive(Debug)]` works only on a value that is never formatted, since nothing can be printed but numbers.
- Everything else on this page works as in Rust.

## Derive

`Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash` work. `Debug` works on a value that
is never formatted.

## Code generation

`#[inline]`, `#[inline(never)]` and `#[cold]` work. `#[inline(never)]` keeps one shared copy of a helper where
rustc would copy it into every caller: see [Items](items.md#functions).

## Type layout

`#[repr]` works, for example `#[repr(i16)]` or `#[repr(u8)]` on a C-like enum.

## Conditional compilation

`#[cfg]` and `cfg!` work.

## Testing and diagnostics

`#[test]` functions can sit beside the program. `#[track_caller]` works.

Feature tests: [attributes](../../tests/features/attributes).
