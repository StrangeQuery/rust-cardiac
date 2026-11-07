# The runtime

[The manual](../manual.md) › The runtime

A program uses the `cardiac` crate (`use cardiac::*;`) for CARDIAC's card reader and teletype and a few built-ins.
Each built-in's full documentation is on [docs.rs](https://docs.rs/cardiac).

## What's different on CARDIAC

- The card reader and the teletype are the whole outside world, and both carry only numbers.
- The `cardiac` crate adds built-ins for things CARDIAC does its own way, such as `digit()`, `skip_cards()` and
  `add_times()`.
- With `cargo run`, the same program reads standard input and prints to standard output, so the two builds can be
  compared.

## The built-ins

| Built-in | On CARDIAC | With `cargo run` |
|----------|------------|------------------|
| `inp() -> i16` | INP: reads the next card | the next number on standard input |
| `out(v: i16)` | OUT: outputs a value | prints `v` on a line of its own |
| `overlay()` | HRS n: stops for deck n, the next phase, whose code loads over this one's | nothing |
| `load_resource("NAME")` | an array the loader fills with the resource NAME: no code | `resources/NAME`, read when it runs |
| `digit(v, k)` | digit k of v (0 the hundreds, 1 the tens, 2 the units), by a shift | the same, in arithmetic |
| `skip_cards(n)` | reads n cards (0..=999) and drops them, by a jump into a row of `INP 00` | n calls of `inp()` |
| `add_times(c, n)` | `n * c` (n 0..=99), by a jump into a row of `ADD c` | `n * c` |
| `assume(cond)` (unsafe) | `core::hint::assert_unchecked`: no code, so checks it proves can go | `assert!` |
| `Flag` | a `bool` kept as -1 (false) or 0 (true): a test is its sign | a plain `i16` |

**`skip_cards` and `add_times`.** A loop of `inp()`s or of `+= c`, n times, costs 6 or 7 instructions a pass.
`skip_cards(n)` and `add_times(c, n)` jump into a row of ten `INP 00` or nine `ADD c` instead. Each row is a
routine put in once (27 and 34 cells). A call costs 17 and 27 instructions, plus about 1.2 instructions a card and 1
an addition. For a few passes (n below 4 or so) the loop is cheaper.

## The outside world

The card reader and the teletype are the whole outside world. An `extern "C"` block is refused: see
[Items](items.md#external-blocks). Text can't be printed: see [Macros](macros.md).
