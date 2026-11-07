# Macros

[The rust-cardiac manual](../manual.md) › Macros

## What's different on CARDIAC

- `println!`, `print!`, `format_args!` and `write!` are refused, because CARDIAC prints only numbers. Use `out()`.
- `format!` and `vec!` are refused, because there is no heap.
- A panic's message keeps its literal text but not the values a formatted message would print.

## Macros by example

`macro_rules!` works. A macro is expanded by rustc before cargo-cardiac sees the program, so it costs only what
its expansion costs.

## Macros from the standard library

These work:

- `matches!`, `cfg!`, `concat!`, `stringify!`, `file!` and `line!`.
- `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!` and its kin, `panic!()`, `unreachable!()`, `todo!()` and
  `unimplemented!()`, with a literal message or a formatted one. The panic map keeps the message's literal text,
  without the values. See [Panic](panic.md).

These are refused:

| Refused | Why | Instead |
|---------|-----|---------|
| `println!`, `print!`, `format_args!`, `write!` | CARDIAC prints only numbers. | `out()` |
| `format!`, `vec!` | They need the heap, and every value's cell is fixed when the program is built. | Fixed-size arrays |
| `compile_error!` | It stops the build everywhere, natively too. | |

Feature tests: [macros](../../tests/features/macros).
