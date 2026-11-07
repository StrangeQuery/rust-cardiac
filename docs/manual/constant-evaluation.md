# Constant evaluation

[The manual](../manual.md) › Constant evaluation

rustc does all constant evaluation. Anything it folds away first, such as a `const fn`, costs nothing on CARDIAC.

## What's different on CARDIAC

- A constant past a card is refused, even in a branch no run takes.
- `i16::MAX` and `i16::MIN` can't be used as values, though comparing with them works.

## Constants past a card

A constant has to fit on a card, -999 to 999, or the build refuses it with RCE0009. That covers a literal like
`1010`, arithmetic rustc works out at compile time (`101 * 10`), even in a branch no run takes, and `i16::MAX` and
`i16::MIN` used as values. Comparing with `i16::MAX` or `i16::MIN` works.

This is Rust's own rule, applied to a card instead of an `i16`. rustc refuses `30000 * 2` in an `i16`
(`arithmetic_overflow`, an error by default), and `i16::MAX + 1` in a `const`. Like rustc's, the check is only as
good as rustc's folding: `x * 10` with x known to be 101 may be refused at one opt-level and panic as an overflow at
run time at another.

Keep constants on a card, and compute what may pass one at run time, where the overflow check sees it.

Feature tests: [items](../../tests/features/items) and [expressions](../../tests/features/expressions).
