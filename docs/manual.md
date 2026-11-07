# The rust-cardiac manual

This is the manual for writing Rust for CARDIAC with cargo-cardiac. It says what compiles, what it costs in
CARDIAC's 96 cells, and what is refused and why. Its chapters follow the
[Rust Reference](https://doc.rust-lang.org/reference/), so a feature is where you would look for it there. Chapters
of the Reference with nothing particular to CARDIAC are left out.

Every entry was checked by building a small program four ways (opt-level 1 and 2, with overflow checks on
and off) and running it both on CARDIAC and natively with `cargo run`. "Works" means the two print the same. A
refusal is a compile error, never a wrong answer.

## Contents

1. [Lexical structure](manual/lexical-structure.md): literals.
2. [Macros](manual/macros.md): `macro_rules!`, the standard macros, and why `println!` is refused.
3. [Items](manual/items.md): functions and recursion, structs, enums, unions, constants and statics, traits, and
   external blocks.
4. [Attributes](manual/attributes.md): derives, inlining, `repr`, `cfg` and tests.
5. [Statements and expressions](manual/statements-and-expressions.md): operators and what they cost, control flow,
   loops, indexing and closures.
6. [Patterns](manual/patterns.md).
7. [Types](manual/types.md): the integer types and the card's limit, arrays, slices, structs, enums, references,
   function pointers, trait objects, and the heap.
8. [Memory](manual/memory.md): the 96 cells, phases, and the stack.
9. [Panic](manual/panic.md): bounds and overflow checks, and what a panic prints.
10. [Unsafety](manual/unsafety.md): `assume()`, unchecked operations and raw pointers.
11. [Constant evaluation](manual/constant-evaluation.md): constants past a card.
12. [The runtime](manual/runtime.md): `inp()`, `out()` and the other built-ins of the `cardiac` crate.

Appendix: [The core library](manual/library.md): integer methods, `Option` and `Result`, iterators, arrays and
slices, and strings.

## How it fits together

rustc does the whole front end: parsing, type checking, trait resolution, the borrow checker and constant
evaluation. A program rustc rejects never reaches CARDIAC, and its errors are Rust's own. cargo-cardiac compiles
the MIR that rustc writes for the program that's left. So anything rustc folds away first, such as a `const fn`, a
closure called where it's made or a small generic function it inlines, costs nothing and needs no support.

One limit is CARDIAC's, not Rust's: a cell holds -999 to 999. A program whose values fit on a card prints the same
on CARDIAC as natively. What happens past a card is in [Types](manual/types.md#integer-types).

## Index

Everything in the manual, in three groups: what works well, what works but costs more than it looks, and what is
refused. Each area links to its chapter.

### 1. Works well

Use these freely: they compile to what you'd write by hand.

#### [Types](manual/types.md)

- `i16`, the card's own number.
- `i8`, from -128 to 127, wrapping as Rust's does.
- `u8` and `u16`.
- `usize`, for indexes and counters.
- `isize` and `i32`, whose values stay on a card.
- `bool`, and `char`, a cell holding its code point.
- An integer literal with no type (`i32` by default), which works as an `i16` would.
- Literals in hexadecimal, octal and binary, with `_` and a type suffix, and byte literals (`b'A'`).

#### [Arithmetic](manual/statements-and-expressions.md#operators)

- `+`, `-`, unary `-`, and comparisons.
- `*` by a constant (adds and decimal shifts), and `/` and `%` by a power of ten (shifts).
- `!` on integers, and `<<` and `>>` by a constant.
- `digit(v, k)` for one digit of a number, and `add_times(c, n)` for `n * c` by additions (n 0..=99).
- `x & m` for a mask m one below a power of two: `x & 3` is `x.rem_euclid(4)`, a subtracting loop.
- Compound assignment, and destructuring assignment (`(a, b) = (b, a)`).

#### [Integer methods](manual/library.md#integer-methods)

- `abs`, `wrapping_abs`, `unsigned_abs`, `abs_diff`, `signum`, `is_negative` and `is_positive`.
- `min`, `max`, `clamp` and `cmp`.
- `checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_rem`, `checked_neg` and `checked_pow`.
- `wrapping_add`, `wrapping_sub`, `wrapping_mul`, `wrapping_div`, `wrapping_rem` and `wrapping_neg`.
- `saturating_add`, `saturating_sub` and `saturating_mul`.
- `overflowing_add` and `overflowing_sub`.
- `div_euclid` and `rem_euclid`, and `pow`, `isqrt` and `ilog10`.
- On a `u8` or a `usize`, `is_power_of_two` and `next_power_of_two`, and a `u8`'s `midpoint`.
- `from`, `into`, `try_from` and `try_into` between the types above.
- `BITS`, `u8::MIN` and `u8::MAX`.

Where rustc calls one rather than inlining it, the compiler writes the code rustc would have inlined. No card holds
`i16::MIN` or `i16::MAX`, so `abs` has nothing to check, an `i16`'s saturating methods panic past a card as its plain
arithmetic does, and `checked_add` is `None` past 999.

#### [Control flow](manual/statements-and-expressions.md#control-flow)

- `if` and `else`, also as an expression.
- `if let`, let chains, `let else` and `while let`.
- `while`, and `loop`, also with `break value`.
- Labelled blocks, `break` and `continue`, and early `return`.
- `match` on integers, `bool`s, `char`s, enums, structs, tuples and arrays, with ranges, guards, `@` bindings, `|`
  alternatives and nested patterns.
- `for` over a range (`a..b`, `a..=b`, `.rev()`, `.step_by(k)`), over an array, and over `.iter()`, `.iter_mut()`
  and `.iter().enumerate()`.

#### [Data](manual/types.md#arrays)

- Fixed-size arrays of any of the types above, of structs, of tuples and of arrays. `[[i16; 3]; 3]` is one array,
  row after row.
- `const` values and tables, also from other crates, and `static` and `static mut`.
- Tuples, nested too, passed and returned.
- Structs with named fields, tuple structs, unit structs and generic structs, with methods (`impl`), functional
  update and destructuring.
- A `union` read through the field it was written by.
- Shadowing and block expressions.

#### [C-like enums](manual/items.md#enumerations)

- `enum Colour { Red, Green, Blue }`, with numbers given (`Low = -5`, `High = 3 * 300`) or not, and `#[repr(i16)]`
  or `#[repr(u8)]`. One cell, the variant's number.
- `match`, `as i16`, passed and returned.
- `==`, `<` and `cmp` of a derived `PartialEq`, `PartialOrd` or `Ord`.

#### [Enums with data](manual/types.md#enums)

- `enum Shape { Dot, Circle(i16), Rect { w: i16, h: i16 } }`, generic enums of the program's own, `Option` and
  `Result`: built, matched, `if let`, passed and returned, nested.
- `?`, also converting the error with `From`.
- `Option`'s and `Result`'s methods: `is_some`, `unwrap`, `expect`, `unwrap_or`, `unwrap_or_else`,
  `unwrap_or_default`, `map`, `map_or`, `and_then`, `filter`, `ok_or`, `or`, `xor`, `zip`, `take`, `replace`,
  `insert`, `get_or_insert`, `as_ref`, `as_mut`, `copied`, `map_err`, `ok`, `err` and the rest.
- `==` and `<` of two `Option`s.
- `unwrap()` and `expect()` panic with Rust's message (`Result`'s without the `Err`'s value).
- `Option<NonZeroI16>`, `Option<bool>` and `Option<&i16>` use their niche, as Rust does.

An enum with data costs a tag cell: see group 2.

#### [Functions](manual/items.md#functions)

- Parameters and a result.
- Small helpers rustc inlines. `#[inline(never)]` keeps one shared copy.
- Generic functions, inlined or not, and const generics.
- Traits with static dispatch, default methods, associated types and constants, supertraits, generic traits and
  blanket impls.
- Operator overloading (`Add`, `AddAssign`, `Neg`, `PartialEq`, `PartialOrd`).
- `impl Trait` as a parameter and as a result, and `const fn`.
- Methods taking `self`, `&self` or `&mut self`.
- A function's call of itself as the last thing it does, which is a loop. Functions that call one another at their
  ends are one function, so neither needs a stack.

Other recursion has a stack: see group 2.

#### [Closures and indirect calls](manual/statements-and-expressions.md#closures)

- Closures that capture or not, `move` closures and `FnMut` ones: called where they are made, passed to a generic
  function, returned as `impl Fn`, stored in a struct, or behind a `&dyn Fn`.
- Function pointers, chosen at run time, kept in an array, passed and returned.
- `&dyn Trait` of the program's structs, chosen at run time.

#### [References](manual/types.md#references-and-pointers)

- `&mut` and `&` to a scalar, a struct or an enum.
- Arrays by reference, `&[i16; N]` and `&mut [i16; N]`.
- Slices, `&[T]` and `&mut [T]`, of a length known only at run time, subslices (`&t[1..n]`) and slice patterns
  (`[first, .., last]`).
- A reference to a reference, and a reference returned from a function.
- Raw pointers and `p.add(i)`.

#### [Iterators](manual/library.md#iterators)

- On a range or an array's iterator: `sum`, `count`, `fold`, `max`, `min`, `find`, `position`, `all`, `any`, `last`
  and `nth`.
- Adaptor chains of `map`, `filter`, `enumerate`, `skip`, `take`, `rev`, `chain`, `zip` and `step_by`, from a range,
  an array, `once`, `repeat` or `successors`, consumed by `sum`, `count`, `fold`, `for_each`, `max` or `min`. A chain
  is compiled as the one loop it stands for.

#### [Array and slice methods](manual/library.md#arrays-and-slices)

- `len`, `is_empty`, `get`, `first`, `last` and `contains`.
- `fill`, `swap`, `reverse`, `rotate_left`, `sort_unstable` and `binary_search`.
- `split_at`, `copy_from_slice`, `chunks` and `windows`.
- `[T; N]::map` and `core::array::from_fn`.
- `==` of two arrays.

#### [Strings](manual/library.md#strings)

- A string literal's `len()`, its bytes (`as_bytes()`) and indexing. A literal is a cell a byte.
- Byte strings (`b"CARD"`).
- `concat!`, `stringify!`, `file!` and `line!`.

#### [The rest of core](manual/library.md#the-rest-of-core)

- `core::mem::swap`, `replace` and `take`.
- `Ordering` and its methods, `then`, `partial_cmp` and `core::cmp::Reverse`.
- `bool::then` and `then_some`.
- `char::to_digit`, `char::from_digit` and `is_ascii_digit`.
- `core::hint::black_box`, `assert_unchecked` and `unreachable_unchecked`.

#### [Checks](manual/panic.md)

- `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!` and its kin.
- `panic!()`, `unreachable!()`, `todo!()` and `unimplemented!()`, with a literal message or a formatted one. The
  panic map keeps the literal text, without the values.
- Bounds and overflow checks, and `assume()`.
- A check rustc has proved fails (`#[allow(unconditional_panic)]`), which is the panic.

#### [Macros and attributes](manual/macros.md)

- `macro_rules!`, `matches!`, `cfg!` and `#[cfg]`.
- Derives of `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`, and `Debug` on a value
  that is never formatted.
- `#[repr]`, `#[inline]`, `#[cold]` and `#[track_caller]`.
- `#[test]` functions beside the program.

#### [Built-ins](manual/runtime.md)

- `inp()` and `out()`.
- `overlay()`, for a program in phases.
- `load_resource("NAME")`, for data loaded with the deck.
- `digit()`, `skip_cards(n)` and `add_times(c, n)`.

### 2. Works, but costs more than it looks

These compile and give the right answer. Each has a cheaper way, when the cells or the time matter.

| Construct | What it costs | Cheaper |
|-----------|---------------|---------|
| `*` of two variables | A multiply routine, put in once (about 35 cells), and repeated addition: as many steps as the smaller factor. `pow` by a variable is that many multiplications. | Multiply by a constant; keep the smaller factor second. |
| `/` or `%` of two variables | A divide routine, put in once (about 40 cells for `/`, 50 for `%`), and repeated subtraction: as many steps as the quotient. | Divide by a power of ten; `digit()` for one digit. |
| `&`, `\|` and `^` between integers, and shifts by a variable | A routine that reads a card's bits as Rust's `i16` has them, put in once: 66 instructions for `&` (67 for `^`), 49 for two `u8`s, 26 for `>>`, 46 for an `i16`'s `<<`, and a program using one has about 25 cells of its own left. About ten passes of 20 instructions each time. Where Rust's answer is past a card (`512 \| 511` is 1023), the program panics. The build warns about each (RCW0001). | A mask one below a power of two; `*` and `/` by powers of two; `digit()`. |
| The bit methods | `count_ones`, `leading_zeros`, `trailing_zeros` and a `u8`'s rotations walk the cell's bits: about 20 to 40 cells a call, written where it is called, and up to ten passes. The build warns about them too. | A table, or arithmetic on the digits. |
| Recursion other than a call of itself at the end | A stack of its own: about 30 cells of code for each place the function calls itself, and a cell a call for each local still read after the call. The stack is as deep as the memory left over holds, and a call with no room panics with `stack overflow`. Fibonacci's two calls don't fit in 96 cells. | A loop; a call of itself as the last thing it does, which is a loop. |
| A call through a function pointer or a `&dyn Trait` | A test for each function it can be, then a direct call: about two cells a function. A `&dyn` is a tag beside its types' fields, and passing one passes them all. | A direct call; a generic function. |
| A slice of run-time length | Its length in a cell of its own, passed at every call beside the address. | `&[T; N]`, where the length is known. |
| A loop of `inp()`s or of `+= c`, n times | 6 or 7 instructions a pass: the count tested, stored and taken down, then the work. | `skip_cards(n)` and `add_times(c, n)`: a computed jump into a row of ten `INP 00` or nine `ADD c`, a routine put in once (27 and 34 cells), about 1.2 instructions a card and 1 an addition, plus 17 and 27 a call. For a few passes (n below 4 or so) the loop is cheaper. |
| An array passed by value | Copied element by element at every call: two instructions an element, and the parameter's own cells. | Pass `&[T; N]`. |
| A struct passed or returned by value | Each field copied. Cheap for a few fields. | Fine as it is, for small structs. |
| `==`, `!=`, and a `match` on values | CARDIAC's one test is TAC, "jump if negative": `x < k` is a subtraction and a TAC, but `x == k` is two (not below, not above), about six instructions. A `match` tests its arms one after another, each an equality, and a chain of `else if x == k`; a value tested once and then again (`let b = if c { 1 } else { 0 }; if b == 1`) costs both, unless the compiler can follow the constant straight to the arm. `==` of two `Option`s tests both variants and then the payloads. | Test with `<` and `>=` where the values allow (`if x < 0`, ranges in a `match`), the most common arm first; keep a value's sign meaningful (-1 for none: the niche layout does this for `Option<i16>`). |
| An enum with data | A tag cell, the variant's number, beside cells for the fields, which the variants share; setting a variant stores the tag, a `match` tests it. An `Option<i16>` whose payload the range pass can prove is never negative (a count from 0, an index, a card the program `assume()`s is 0 or more) has no tag: `None` is -1 in the payload's cell, as a hand-written sentinel is, so `is_none()` is one TAC and `unwrap_or(-1)` is the cell itself. Within one function rustc usually takes an `Option` apart itself. At opt-level 1, where rustc calls the methods, a few cells more than at opt-level 2. | `assume()` what the deck guarantees (`assume(c >= 0)`), so the niche can be proven. |
| `.rem_euclid()` by a variable | The divide routine, as for `%`, and about 10 cells more to make a negative remainder positive. | By a constant: a power of ten is shifts, as for `%`. |
| An array or pointer read through a variable index | Self-modifying code: the instruction is built from the address, stored and run, a few cells and steps each time. An array of arrays costs two to five instructions more an index, as `t[r * W + c]` by hand does. | Unavoidable on CARDIAC (no indexed addressing); hoist a repeated `t[i]` into a local. |
| Overflow checks on (`dev`) | Each checked `i16` sum calls a shared check routine, and every checked operation is a test and a branch. An iterator's `sum` is checked whatever the build's setting, as core's is. | Build with `--release` once the program is right. |
| Arrays of structs | One array per field, and an element passed whole copies each field. | Parallel arrays, one per field. |
| A struct or an enum behind a `&mut` | A `&mut` of each of its cells: the call passes an address a cell, and each read or write through one is self-modifying code, a few cells and steps. | The value passed in and returned, for a function that touches every field. |
| `let x = if c { e } else { K }` in straight-line code | Each arm sets x and one jumps past the other: about 3 cells more than `let mut x = K; if c { x = e; }`, whose K is x's initial value in the deck when it runs once. The compiler doesn't do this itself: storing K first keeps x alive across the test, which can cost a shared cell more than it saves. | `let mut x = K; if c { x = e; }`, in code that runs once. |
| An inlined helper called in many places | Its code copied into each. | `#[inline(never)]` keeps one copy. |
| `opt-level = 0` | rustc's unoptimised MIR, and the compiler's own passes off: most programs don't fit in 96 cells. | opt-level 1 or more (cargo's `dev` profile is fine). |

### 3. Refused: these don't compile

The build stops with an error, and `cargo cardiac help CODE` says what its code means and what to write instead.

#### What CARDIAC can't do

These need something CARDIAC doesn't have, so no work in the compiler would make them right. A cell holds a sign
and three decimal digits, the program and its data share 96 of them, and the card reader and the teletype are the
whole outside world.

| Area | Refused | Instead |
|------|---------|---------|
| [**Wide integers**](manual/types.md#integer-types) | `i64`, `u32`, `u64`, `i128`, `u128`. A cell holds 1,999 values, and wider types exist to hold values a card never could, so accepting them would only move the panic. An `i32` is accepted under the same rule as an `i16`, so one past a card can't be held even for a moment: `50 * 50 / 10` panics with overflow checks on, where Rust prints 250. | `i16`, with values kept on a card. |
| [**Floats**](manual/types.md#floating-point-types) | `f32` and `f64`. A cell holds whole numbers, with no fractions and no exponent. | Scaled integers: tenths in a cell are a number ten times as big. |
| [**The heap**](manual/types.md#the-heap) | `Box`, `Vec`, `String`, `vec!`, `format!`, and a recursive enum through `Box`. Every value's cell is fixed when the program is built. | Fixed-size arrays. |
| [**Text**](manual/macros.md#macros-from-the-standard-library) | `println!`, `print!`, `format_args!` and `write!`: CARDIAC prints only numbers. Comparing strings (two short literals compared come to about 124 cells) and parsing them (`i16::from_str_radix`): a program's input is numbers on cards. | `out()`; numbers in place of words. |
| [**The outside world**](manual/runtime.md#the-outside-world) | An `extern "C"` block: there is nothing on the other side of the call. (An `extern "C" fn` defined in Rust works.) | A Rust function. |
| [**Byte order**](manual/library.md#integer-methods) | `swap_bytes`: 100 becomes 25,600, far past a card. | |
| [**Rust's own rules**](manual/patterns.md) | `let Some(x) = o;`, which rustc refuses (E0005), and `compile_error!`. They build nowhere. | `let else`; `if let`. |

#### Refused on purpose

| Area | Refused | Instead |
|------|---------|---------|
| [**Constants**](manual/constant-evaluation.md) | A constant past what a card holds (-999 to 999): a literal like `1010`, or arithmetic rustc works out at compile time (`101 * 10`), even in a branch no run takes, and `i16::MAX` and `i16::MIN` used as values (comparing with one works). It is Rust's own rule, for a card instead of an `i16`: rustc refuses `30000 * 2` in an `i16` (`arithmetic_overflow`, an error by default) and `i16::MAX + 1` in a `const`. Like rustc's, it is as good as rustc's folding: `x * 10` with x known to be 101 may be refused at one opt-level and panic as an overflow at run time at another. | Keep constants on a card; compute what may pass one at run time, where the overflow check sees it. |
| [**`i16::midpoint`**](manual/library.md#integer-methods) | rustc adds the two in `i32` and halves, and past a card that sum can't be held, so the answer would be wrong. (A `u8`'s works.) | `(a + b) / 2`, where the sum stays on a card. |

#### Too big for 96 cells

| Area | Refused | Instead |
|------|---------|---------|
| [**Shifts**](manual/statements-and-expressions.md#operators) | A program that uses both `<<` and `>>` by a variable: the two routines don't fit with anything else. | One of them, or shifts by a constant. |
| [**`&dyn Trait`**](manual/types.md#trait-objects) | An object chosen at run time whose methods multiply, with overflow checks on (117 cells; 89 without them). | Build with overflow checks off; a `match` on an enum. |

#### Not done yet

| Area | Refused | Instead |
|------|---------|---------|
| [**Iterators**](manual/library.md#iterators) | `filter_map`, `flat_map`, `peekable`, `scan`, `take_while`; `any`, `all`, `find` and `position` on a chain of adaptors; a chain kept and stepped by hand, or in a `for` loop; `rev` of `enumerate`, `skip`, `take`, `zip`, `step_by` or `successors`. | A `for` loop over a range; the method on a range or a plain array iterator. |
| [**Recursion**](manual/items.md#functions) | A recursive function with a reference or a pointer among its locals or parameters, one that returns a struct, or one that keeps a local other than a number or a bool across the call; recursion through more than one function, unless every call is at its end. | A loop; values passed in and returned. |
| [**Closures and objects**](manual/types.md#trait-objects) | A closure called through `FnOnce`, or one that captures a field of a struct; a closure that captures nothing behind `&dyn Fn`; `&mut dyn`, `Box<dyn ..>`, an object of a primitive or a generic struct, and a trait's default method called through an object. A function pointer compared, or kept in a struct, a tuple or an `Option`. | Generics; a function pointer; a `match` on an enum. |
| [**Bits**](manual/library.md#integer-methods) | `<<` of an `i8` or a `usize`; rotations of an `i16`, or by a variable; a `usize`'s `leading_zeros`. | An `i16` or a `u8`. |
| [**Arrays of arrays**](manual/types.md#arrays) | A row borrowed (`&t[r]`, `t[r].iter()`) or copied out (`let row = t[1]`). | `t[r][c]` one element at a time. |
| [**Data**](manual/types.md#enums) | An enum with data in a struct that is itself in an enum (`Option<Card>`, `Card` holding an `Option`); a struct with an array field behind a `&mut`. | The enum by value; a struct of the fields and a tag; the struct passed in and returned. |

## How this was checked

The [feature tests](../tests/features/SUPPORT.md) are about 750 small programs, one feature each, in the Rust
Reference's order. Each is built and run as described at the top, and `./test_features.sh` remakes the table from
what happened. The test programs in `tests/programs` and the examples cover the same ground at more length
(`./test_compiler.sh`, `./test_examples.sh`).

Probing this way found two miscompiles, both fixed: a loop whose condition starts with a call that isn't inlined
ran its body once (jump threading went through the call), and an array passed by value got only its first element.
Two more were fixed in 0.2.0: a value borrowed for a call could share a cell with another value and be read
wrongly, and an overflow-checked sum of three terms could lose its first term when another of them was an array
element at an index worked out at run time.
