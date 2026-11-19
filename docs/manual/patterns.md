# Patterns

[The rust-cardiac manual](../manual.md) › Patterns

## What's different on CARDIAC

- A `match` tests its arms one after another, so put the most common arm first. A `match` with arms on enough
  consecutive values (`0 => .., 1 => .., 2 => ..`, an enum's variants) jumps straight to its arm instead.
- Ranges are cheaper to test than equalities.
- Otherwise patterns work as in Rust.

## Where patterns work

Patterns work in `match`, `if let`, `while let`, `let else` and `let`. They can match integers, `bool`s, `char`s,
enums, structs, tuples and arrays, with ranges, guards, `@` bindings, `|` alternatives and nested patterns.

Slice patterns work on arrays and slices: `[first, .., last]`, `[a, ..]`, `[]`, and `[x, rest @ ..]`.

A `match` tests its arms one after another, so the cost of an arm depends on its pattern. A `match` whose arms are
on consecutive values is a jump into a table of jumps when that takes fewer cells: about six values or more when
the range of the value is known (its type, a mask, `% 10`, an `assume`), nine or more when it isn't. Then every arm
costs the same, five instructions, and its order doesn't matter. A range is tested with `<`
and `>=`, which CARDIAC does directly, while an equality takes two tests: see
[Statements and expressions](statements-and-expressions.md#control-flow).

**Refused by rustc:** `let Some(x) = o;` is a refutable pattern in a plain `let`, which rustc refuses (E0005) and
which builds nowhere. Use `let else` or `if let`.

Feature tests: [patterns](../../tests/features/patterns).
