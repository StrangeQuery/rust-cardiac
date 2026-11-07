# Attributes

[The manual](../manual.md) › Attributes

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
