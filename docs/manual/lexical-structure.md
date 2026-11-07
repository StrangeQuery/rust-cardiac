# Lexical structure

[The rust-cardiac manual](../manual.md) › Lexical structure

## What's different on CARDIAC

- Every integer literal has to fit on a card, -999 to 999. `1010` is refused, though it is a fine `i16` in Rust.
- A `char` is one cell holding its code point, so ASCII and Latin-1 characters fit.
- A string literal takes a cell a byte, and it can't be printed: CARDIAC prints only numbers.

## Literals

**Integer literals** work in decimal, hexadecimal, octal and binary, with `_` and a type suffix. A literal with no
type (`i32` by default) works as an `i16` would. Every literal has to fit on a card, -999 to 999, or the build
refuses it: see [Constant evaluation](constant-evaluation.md).

**Byte literals** (`b'A'`) work, and so do **`char` literals**. A `char` is a cell holding its code point,
so every ASCII and Latin-1 character fits.

**String literals** work for their length, their bytes and indexing: `s.len()`, `s.as_bytes()`, `s.as_bytes()[i]`.
A literal takes one cell a byte. **Byte strings** (`b"CARD"`) work the same way. CARDIAC prints only numbers, so a
string can't be printed, compared or parsed: see [Macros](macros.md) and [The core library](library.md#strings).

Feature tests: [types](../../tests/features/types) (the literal rows) and
[expressions](../../tests/features/expressions).
