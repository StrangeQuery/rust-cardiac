# Memory

[The rust-cardiac manual](../manual.md) › Memory

## What's different on CARDIAC

- The program and its data share 96 cells. There is no separate memory for either.
- There is no heap, and a stack only when recursion needs one.
- A program too big for 96 cells can run in phases, each loading its code over the last one's.

## Cells

CARDIAC has 100 cells. The program and its data share 96 of them, cells 03 to 98, per phase. Every value's cell is
fixed when the program is built, so there is no heap: see [Types](types.md#the-heap). A program that doesn't fit
is refused with RCE0032.

## Phases

A program bigger than memory can come in phases: `NAME.cqb`, then `NAME.2.cqb`, and so on. The data is shared, and
each phase's code is loaded over the one before. `overlay()` in `main` says where a phase ends, or the compiler
places the phases itself when the package turns `auto-overlay` on:

```toml
[package.metadata.cardiac]
auto-overlay = true
```

## The stack

There is no stack unless the program needs one. A function's call of itself at its end is a loop, and functions
that call one another at their ends are one function. Other recursion gets a stack of its own, as deep as the memory
left over holds: see [Items](items.md#functions).

## What makes a program bigger

- An inlined helper called in many places is copied into each one. `#[inline(never)]` keeps one copy.
- The routines for `*`, `/` and `%` of two variables and for the bit operators are put in once, and are large: see
  [Statements and expressions](statements-and-expressions.md#operators).
- Overflow checks add a test and a branch to every checked operation: see [Panic](panic.md).
- `opt-level = 0` gives rustc's unoptimised MIR, and the compiler's own passes are off, so most programs don't fit in
  96 cells. Use opt-level 1 or more. Cargo's `dev` profile is fine.

Feature tests: [items](../../tests/features/items) and [types](../../tests/features/types).
