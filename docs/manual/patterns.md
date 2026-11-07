# Patterns

[The manual](../manual.md) › Patterns

Patterns work in `match`, `if let`, `while let`, `let else` and `let`. They can match integers, `bool`s, `char`s,
enums, structs, tuples and arrays, with ranges, guards, `@` bindings, `|` alternatives and nested patterns.

Slice patterns work on arrays and slices: `[first, .., last]`, `[a, ..]`, `[]`, and `[x, rest @ ..]`.

A `match` tests its arms one after another, so the cost of an arm depends on its pattern. A range is tested with `<`
and `>=`, which CARDIAC does directly, while an equality takes two tests: see
[Statements and expressions](statements-and-expressions.md#control-flow).

**Refused by rustc:** `let Some(x) = o;` is a refutable pattern in a plain `let`, which rustc refuses (E0005) and
which builds nowhere. Use `let else` or `if let`.

Feature tests: [patterns](../../tests/features/patterns).
