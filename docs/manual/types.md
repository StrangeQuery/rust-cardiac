# Types

[The rust-cardiac manual](../manual.md) › Types

## What's different on CARDIAC

- A cell holds -999 to 999, and so does every integer type CARDIAC takes. The accumulator has a fourth digit, but a
  program can't use it: past a card, a value panics with overflow checks on, or keeps its last three digits with them
  off.
- No integer type is wider than a card. `i32` and `isize` are held as an `i16` is, and `i64`, `u32`, `u64`, `i128` and
  `u128` are refused.
- There are no floats and no heap: `f32`, `f64`, `Box`, `Vec` and `String` are refused.
- An enum with data costs a tag cell, unless its payload leaves a value free for `None`.

## Integer types

A CARDIAC cell holds a sign and three decimal digits, -999 to 999. That is the one limit that is CARDIAC's rather
than Rust's.

| Type | On CARDIAC |
|------|------------|
| `i16` | -999 to 999, the card's own number. |
| `i8` | -128 to 127, wrapping as Rust's does. |
| `u8` | 0 to 255, wrapping as Rust's does (`0u8 - 1` is 255). |
| `u16` | 0 to 999. |
| `usize` | For indexes and counters. |
| `isize`, `i32` | -999 to 999, held as an `i16` is. |

**Past a card.** With overflow checks on, an `i16` past -999..=999 panics on CARDIAC, where native Rust carries on
to 32767. With them off, CARDIAC keeps the last three digits (1100 is 100) where Rust would keep the whole value. A
program whose values fit on a card prints the same both ways.

Without overflow checks, a `u8` or `i8` product that passes a card is what CARDIAC keeps of it, as for an `i16`:
the last three digits with the sign (40000 keeps 000), which the type then wraps. A product from -999 to 999 wraps
as Rust's does.

A `usize` past a card, as `-1i16 as usize` makes one (`usize::MAX`), or `0usize - 1` without overflow checks, is
held as the negative number it is short of 2^64 by. It compares above every other `usize`, casts back to `i16` or
to `u8` as in Rust, can be added to and subtracted from, and as an index fails the bounds check. Anything else done
with it panics, as a value past a card does: passing it to a function, multiplying, dividing, or an
overflow-checked `+` or `-`.

**Refused, because CARDIAC can't hold them:** `i64`, `u32`, `u64`, `i128` and `u128`. A cell holds 1,999 values,
and wider types exist to hold values a card never could, so accepting them would only move the panic. An `i32` is
accepted under the same rule as an `i16`, so one past a card can't be held even for a moment: `50 * 50 / 10` panics
with overflow checks on, where Rust prints 250. Use `i16`, with values kept on a card.

## Floating-point types

`f32` and `f64` are refused. A cell holds whole numbers, with no fractions and no exponent. Use scaled integers:
tenths in a cell are a number ten times as big.

## Boolean and char

`bool` works. So does `char`, a cell holding its code point. For a flag kept in a cell and tested many times, the
`cardiac` crate's `Flag` saves an instruction a test: see [The runtime](runtime.md).

## Arrays

Fixed-size arrays work, of any of the types on this page, of structs, of tuples and of arrays. An array of arrays
(`[[i16; 3]; 3]`) is one array. Where the program still fits that way, it is laid out by tens: each row starts ten
cells after the one before, and the program's other cells go in the holes at the ends of the rows. Otherwise it is
row after row. Either way Rust's bounds checks are kept, so an index past a row's end panics, and nothing reads a
hole. For what indexing costs, see
[Statements and expressions](statements-and-expressions.md#index-expressions).

Not done yet: a row of an array of arrays borrowed (`&t[r]`, `t[r].iter()`) or copied out (`let row = t[1]`). Use
`t[r][c]` one element at a time.

## Slices

`&[T]` and `&mut [T]` of a length known only at run time work, and so do subslices (`&t[1..n]`) and slice patterns.
A slice's length is kept in a cell of its own and passed at every call beside the address. Where the length is
known, `&[T; N]` is cheaper.

## Tuples

Tuples work, nested too, and can be passed and returned.

## Structs

An array of structs is one array per field, and an element passed whole copies
each field; parallel arrays, one per field, are cheaper. For passing structs, see [Items](items.md#functions).

Not done yet: a struct with an array field behind a `&mut`. Pass the struct in and return it.

## Enums

A C-like enum is one cell, the variant's number.

An enum with data has a tag cell, the variant's number, beside cells for the fields, which the variants share.
Setting a variant stores the tag, and a `match` tests it. Within one function rustc usually takes an `Option` apart
itself. At opt-level 1, where rustc calls the methods, an enum costs a few cells more than at opt-level 2.

An `Option<i16>` whose payload the range pass can prove is never negative has no tag: `None` is -1 in the payload's
cell, as a hand-written sentinel is, so `is_none()` is one TAC and `unwrap_or(-1)` is the cell itself. A count from
0, an index, or a card the program `assume()`s is 0 or more can all be proved. `Option<NonZeroI16>`,
`Option<bool>` and `Option<&i16>` use their niche, as in Rust. To get the niche, `assume()` what the deck
guarantees (`assume(c >= 0)`).

Not done yet: an enum with data in a struct that is itself in an enum (`Option<Card>`, with `Card` holding an
`Option`). Pass the enum by value, or use a struct of the fields and a tag.

## Unions

A `union` read through the field it was written by works.

## References and pointers

`&` and `&mut` work to a scalar, a struct or an enum; arrays by reference (`&[i16; N]` and `&mut [i16; N]`); a
reference to a reference; and a reference returned from a function. Raw pointers work, with `p.add(i)`.

## Function pointers

Function pointers work: chosen at run time, kept in an array, passed and returned. A call through one tests each
function it can be, then calls it directly: about two cells a function.

Not done yet: a function pointer compared, or kept in a struct, a tuple or an `Option`.

## Trait objects

`&dyn Trait` of the program's structs, chosen at run time, works. A `&dyn` is a tag beside its types' fields, and
passing one passes them all. A call through one tests each type it can be, then calls directly: about two cells a
function. A direct call or a generic function is cheaper.

Too big for 96 cells: an object chosen at run time whose methods multiply, with overflow checks on (117 cells, or 89
without them). Build with overflow checks off, or use a `match` on an enum.

Not done yet: `&mut dyn`, `Box<dyn ..>`, an object of a primitive or of a generic struct, and a trait's default
method called through an object.

## The heap

`Box`, `Vec`, `String` and a recursive enum through `Box` are refused. Every value's cell is fixed when the program
is built. Use fixed-size arrays.

Feature tests: [types](../../tests/features/types).
