# Statements and expressions

[The rust-cardiac manual](../manual.md) › Statements and expressions

## What's different on CARDIAC

- CARDIAC can only add and subtract. `*`, `/` and `%` of two variables are routines that loop. By a constant, or by a
  power of ten, they are cheap.
- CARDIAC works in decimal, so `&`, `|`, `^` and shifts by a variable are slow routines over the bits. The build warns
  about them (RCW0001).
- CARDIAC's only test is "jump if negative", so `x == k` costs about twice what `x < k` does.
- There is no indexed addressing. Reading `t[i]` builds an instruction and runs it.

## Statements

`let`, `let else`, shadowing and block expressions work.

`let x = if c { e } else { K }` in straight-line code costs about 3 cells more than
`let mut x = K; if c { x = e; }`. Each arm of the `if` sets x and one jumps past the other, while in the second form
K is x's starting value in the deck. That only holds in code that runs once. The compiler doesn't make this change
itself, because storing K first keeps x alive across the test, which can cost a shared cell more than it saves.

## Operators

**Works well:** `+`, `-`, unary `-` and comparisons. `*` by a constant, which becomes additions and decimal shifts.
`/` and `%` by a power of ten, which are shifts. `!` on integers, and `<<` and `>>` by a constant. `x & m` for a
mask one below a power of two: `x & 3` is `x.rem_euclid(4)`, a subtracting loop. Compound assignment, and
destructuring assignment (`(a, b) = (b, a)`).

**Works, but costs more than it looks:**

| Construct | What it costs | Cheaper |
|-----------|---------------|---------|
| `*` of two variables | A multiply routine, put in once (about 35 cells), and repeated addition: as many steps as the smaller factor. `pow` by a variable is that many multiplications. | Multiply by a constant; keep the smaller factor second. |
| `/` or `%` of two variables | A divide routine, put in once (about 40 cells for `/`, 50 for `%`), and repeated subtraction: as many steps as the quotient. | Divide by a power of ten; `digit()` for one digit. |
| `&`, `\|` and `^` between integers, and shifts by a variable | A routine that reads a card's bits as Rust's `i16` has them, put in once: 66 instructions for `&` (67 for `^`), 49 for two `u8`s, 26 for `>>`, 46 for an `i16`'s `<<`, and a program using one has about 25 cells of its own left. About ten passes of 20 instructions each time. Where Rust's answer is past a card (`512 \| 511` is 1023), the program panics. The build warns about each (RCW0001). | A mask one below a power of two; `*` and `/` by powers of two; `digit()`. |
| `==` and `!=` | CARDIAC's one test is TAC, "jump if negative". `x < k` is a subtraction and a TAC, but `x == k` is two tests (not below, not above), about six instructions. A value tested once and then again (`let b = if c { 1 } else { 0 }; if b == 1`) costs both, unless the compiler can follow the constant straight to the arm. | Test with `<` and `>=` where the values allow (`if x < 0`); keep a value's sign meaningful. |

**Too big for 96 cells:** a program that uses both `<<` and `>>` by a variable. The two routines don't fit with
anything else. Use one of them, or shifts by a constant.

**Not done yet:** `<<` of an `i8` or a `usize`. Use an `i16` or a `u8`.

Without overflow checks, arithmetic that passes a card keeps the last three digits: see
[Types](types.md#integer-types). With them, it panics: see [Panic](panic.md).

## Control flow

`if` and `else`, also as an expression; `if let`, let chains and `while let`; `while`; `loop`, also with
`break value`; labelled blocks, `break` and `continue`; and early `return` all work.

**`match`** works on integers, `bool`s, `char`s, enums, structs, tuples and arrays: see [Patterns](patterns.md). A
`match` on values tests its arms one after another, each an equality, as a chain of `else if x == k` does. Ranges
in a `match` are cheaper than equalities, and the most common arm should come first.

## Loops

`for` works over a range (`a..b`, `a..=b`, `.rev()`, `.step_by(k)`), over an array, and over `.iter()`,
`.iter_mut()` and `.iter().enumerate()`. For iterator chains, see [The core library](library.md#iterators).

A loop that does one thing n times costs 6 or 7 instructions a pass: the count tested, stored and taken down, then
the work. For a loop of `inp()`s or of `+= c`, `skip_cards(n)` and `add_times(c, n)` are cheaper: see
[The runtime](runtime.md).

## Index expressions

CARDIAC has no indexed addressing, so an array or pointer read through a variable index is self-modifying code:
the instruction is built from the address, stored and run, a few cells and steps each time. An array of arrays
costs a few instructions more an index: laid out by tens, the row is one decimal shift (`t[10 * r + c]`), and row
after row it is r added W times, as `t[r * W + c]` by hand does. `t[r + 1][c + 2]` is found from `10 * r + c`, with
the 12 in the instruction's address, unless r or c may be a usize past a card (`inp() as usize` without overflow
checks). Hoist a repeated `t[i]` into a local.

## Closures

Closures that capture or not, `move` closures and `FnMut` ones work. They can be called where they are made, passed
to a generic function, returned as `impl Fn`, stored in a struct, or put behind a `&dyn Fn`. A call through a
`&dyn Fn` costs as a call through any trait object does: see [Types](types.md#trait-objects).

Not done yet: a closure called through `FnOnce`, or one that captures a field of a struct; a closure that captures
nothing, behind `&dyn Fn`. Use generics or a function pointer.

Feature tests: [statements](../../tests/features/statements) and [expressions](../../tests/features/expressions).
