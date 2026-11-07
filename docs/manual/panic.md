# Panic

[The manual](../manual.md) › Panic

## What's different on CARDIAC

- A panic outputs -999 and halts. With the panic map, burn says why, as Rust does.
- Overflow means leaving -999 to 999, the range of a card, not the range of the type.
- Overflow checks cost cells on every checked operation, so build with `--release` once the program is right.

## What a panic does

A failed check outputs -999 and halts. With the panic map beside the deck, burn then says why, as a Rust panic
does, and exits 101 (`panicked in main: index out of bounds`). Natively, every panic prints -999 first too, so the
two builds print the same.

## The checks

**Bounds checks are always on**, as in Rust.

**Overflow checks follow the profile:** `overflow-checks`, which is on in `dev` and off in `release`. With them on,
arithmetic that leaves -999..=999 panics. With them off, a cell keeps the last three digits and the program carries
on: see [Types](types.md#integer-types).

Overflow checks cost cells and time. Each checked `i16` sum calls a shared check routine, and every checked
operation is a test and a branch. An iterator's `sum` is checked whatever the build's setting, as core's is. Build
with `--release` once the program is right.

## Panicking macros

`assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!` and its kin, `panic!()`, `unreachable!()`, `todo!()` and
`unimplemented!()` all work, with a literal message or a formatted one. The panic map keeps the message's literal
text, without the values. `unwrap()` and `expect()` panic with Rust's message (`Result`'s without the `Err`'s
value). A check rustc has proved fails (`#[allow(unconditional_panic)]`) is the panic.

## Aborts

The standard library's own checks of unsafe preconditions, which debug assertions keep, work too: `unchecked_add`
and its kin, `assert_unchecked` and `unreachable_unchecked`. A violated one outputs -999 and halts, and burn exits
134, as Rust aborts (`unsafe precondition(s) violated: usize::unchecked_sub cannot overflow`). The ones a `for`
loop's own test proves cost nothing. See [Unsafety](unsafety.md).

Feature tests: [macros](../../tests/features/macros) and [expressions](../../tests/features/expressions).
