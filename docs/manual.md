# rust-cardiac reference guide

This is the manual for writing Rust for CARDIAC with cargo-cardiac. It says what compiles, what it costs in
CARDIAC's 96 cells, and what is refused and why. Its chapters follow the
[Rust Reference](https://doc.rust-lang.org/reference/), so a feature is where you would look for it there. Chapters
of the Reference with nothing particular to CARDIAC are left out.

Every entry was checked by building a small program four ways (opt-level 1 and 2, with overflow checks on
and off) and running it both on CARDIAC and natively with `cargo run`. "Works" means the two print the same. A
refusal is a compile error, never a wrong answer.

## rust-cardiac at a glance

Everything in the manual, area by area: what works well, what costs more than it looks, and what is refused. Each
area's name links to its chapter.

**Works well** compiles to what you'd write by hand. **Costs more than it looks** compiles and gives the right
answer, but has a cheaper way when the cells or the time matter. **Refused** stops the build with an error, and
`cargo cardiac help CODE` says what its code means and what to write instead. A refusal is one of four kinds:

- **CARDIAC can't:** it needs something CARDIAC doesn't have, so no work in the compiler would make it right. A cell
  holds a sign and three decimal digits, the program and its data share 96 of them, and the card reader and the
  teletype are the whole outside world.
- **On purpose:** it could compile, but the answer would be wrong or misleading.
- **Too big:** it doesn't fit in 96 cells.
- **Not done yet:** work for the compiler that hasn't been done.

### [Types](manual/types.md)

**Works well**

- `i16`, from -999 to 999, what a cell holds, not Rust's -32768 to 32767. The accumulator has a fourth digit, but a
  program can't use it: past a card, a value panics with overflow checks on, or keeps its last three digits with them
  off.
- `i8`, from -128 to 127, wrapping as Rust's does.
- `u8`, and `u16` from 0 to 999.
- `usize`, for indexes and counters.
- `isize` and `i32`, held as an `i16` is: -999 to 999.
- `bool`, and `char`, a cell holding its code point.
- An integer literal with no type (`i32` by default), which works as an `i16` would.
- Literals in hexadecimal, octal and binary, with `_` and a type suffix, and byte literals (`b'A'`).

**Refused**

- `i64`, `u32`, `u64`, `i128` and `u128` (CARDIAC can't). A cell holds 1,999 values, and wider types exist to hold
  values a card never could, so accepting them would only move the panic. An `i32` is accepted under the same rule
  as an `i16`, so one past a card can't be held even for a moment: `50 * 50 / 10` panics with overflow checks on,
  where Rust prints 250. Use `i16`, with values kept on a card.
- `f32` and `f64` (CARDIAC can't). A cell holds whole numbers, with no fractions and no exponent. Use scaled
  integers: tenths in a cell are a number ten times as big.

### [Arithmetic](manual/statements-and-expressions.md#operators)

**Works well**

- `+`, `-`, unary `-`, and comparisons.
- `*` by a constant (adds and decimal shifts), and `/` and `%` by a power of ten (shifts).
- `!` on integers, and `<<` and `>>` by a constant.
- `digit(v, k)` for one digit of a number, and `add_times(c, n)` for `n * c` by additions (n 0..=99).
- `x & m` for a mask m one below a power of two: `x & 3` is `x.rem_euclid(4)`, a subtracting loop.
- Compound assignment, and destructuring assignment (`(a, b) = (b, a)`).

**Costs more than it looks**

- `*` of two variables: a multiply routine, put in once (about 35 cells), and repeated addition, as many steps as
  the smaller factor. `pow` by a variable is that many multiplications. Multiply by a constant instead, or keep the
  smaller factor second.
- `/` or `%` of two variables: a divide routine, put in once (about 40 cells for `/`, 50 for `%`), and repeated
  subtraction, as many steps as the quotient. Divide by a power of ten instead, or use `digit()` for one digit.
- `&`, `|` and `^` between integers, and shifts by a variable: a routine that reads a card's bits as Rust's `i16`
  has them, put in once. It is 66 instructions for `&` (67 for `^`), 49 for two `u8`s, 26 for `>>` and 46 for an
  `i16`'s `<<`, and a program using one has about 25 cells of its own left. Each use is about ten passes of 20
  instructions. Where Rust's answer is past a card (`512 | 511` is 1023), the program panics. The build warns about
  each (RCW0001). Use a mask one below a power of two, `*` and `/` by powers of two, or `digit()`.

**Refused**

- A program that uses both `<<` and `>>` by a variable (too big): the two routines don't fit with anything else. Use
  one of them, or shifts by a constant.
- `<<` of an `i8` or a `usize` (not done yet). Use an `i16` or a `u8`.

### [Integer methods](manual/library.md#integer-methods)

**Works well**

- `abs`, `wrapping_abs`, `unsigned_abs`, `abs_diff`, `signum`, `is_negative` and `is_positive`.
- `min`, `max`, `clamp` and `cmp`.
- `checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_rem`, `checked_neg` and `checked_pow`.
- `wrapping_add`, `wrapping_sub`, `wrapping_mul`, `wrapping_div`, `wrapping_rem` and `wrapping_neg`.
- `saturating_add`, `saturating_sub` and `saturating_mul`.
- `overflowing_add` and `overflowing_sub`.
- `div_euclid` and `rem_euclid`, and `pow`, `isqrt` and `ilog10`.
- On a `u8` or a `usize`, `is_power_of_two` and `next_power_of_two`, and a `u8`'s `midpoint`.
- `from`, `into`, `try_from` and `try_into` between the integer types that work.
- `BITS`, `u8::MIN` and `u8::MAX`.

Where rustc calls one rather than inlining it, the compiler writes the code rustc would have inlined. No card holds
`i16::MIN` or `i16::MAX`, so `abs` has nothing to check, an `i16`'s saturating methods panic past a card as its plain
arithmetic does, and `checked_add` is `None` past 999.

**Costs more than it looks**

- `count_ones`, `leading_zeros`, `trailing_zeros` and a `u8`'s rotations walk the cell's bits: about 20 to 40 cells
  a call, written where it is called, and up to ten passes. The build warns about them too. Use a table, or
  arithmetic on the digits.
- `.rem_euclid()` by a variable: the divide routine, as for `%`, and about 10 cells more to make a negative
  remainder positive. By a constant it is cheap: a power of ten is shifts, as for `%`.

**Refused**

- `i16::midpoint` (on purpose). rustc adds the two in `i32` and halves, and past a card that sum can't be held, so
  the answer would be wrong. A `u8`'s works. Use `(a + b) / 2`, where the sum stays on a card.
- `swap_bytes` (CARDIAC can't): 100 becomes 25,600, far past a card.
- Rotations of an `i16`, or by a variable, and a `usize`'s `leading_zeros` (not done yet). Use an `i16` or a `u8`.

### [Constants](manual/constant-evaluation.md)

**Works well**

- `const` values and tables, also from other crates.
- Comparing with `i16::MAX` or `i16::MIN`.

**Refused**

- A constant past what a card holds, -999 to 999 (on purpose). That covers a literal like `1010`, arithmetic rustc
  works out at compile time (`101 * 10`), even in a branch no run takes, and `i16::MAX` and `i16::MIN` used as
  values. It is Rust's own rule, for a card instead of an `i16`: rustc refuses `30000 * 2` in an `i16`
  (`arithmetic_overflow`, an error by default) and `i16::MAX + 1` in a `const`. Like rustc's, it is as good as
  rustc's folding: `x * 10` with x known to be 101 may be refused at one opt-level and panic as an overflow at run
  time at another. Keep constants on a card, and compute what may pass one at run time, where the overflow check
  sees it.

### [Control flow](manual/statements-and-expressions.md#control-flow)

**Works well**

- `if` and `else`, also as an expression.
- `if let`, let chains, `let else` and `while let`.
- `while`, and `loop`, also with `break value`.
- Labelled blocks, `break` and `continue`, and early `return`.
- `match` on integers, `bool`s, `char`s, enums, structs, tuples and arrays, with ranges, guards, `@` bindings, `|`
  alternatives and nested patterns.
- `for` over a range (`a..b`, `a..=b`, `.rev()`, `.step_by(k)`), over an array, and over `.iter()`, `.iter_mut()`
  and `.iter().enumerate()`.

**Costs more than it looks**

- `==`, `!=`, and a `match` on values. CARDIAC's one test is TAC, "jump if negative": `x < k` is a subtraction and a
  TAC, but `x == k` is two tests (not below, not above), about six instructions. A `match` tests its arms one after
  another, each an equality, as a chain of `else if x == k` does. A value tested once and then again
  (`let b = if c { 1 } else { 0 }; if b == 1`) costs both, unless the compiler can follow the constant straight to
  the arm. Test with `<` and `>=` where the values allow (`if x < 0`, ranges in a `match`), put the most common arm
  first, and keep a value's sign meaningful (-1 for none: the niche layout does this for `Option<i16>`).
- A loop of `inp()`s or of `+= c`, n times: 6 or 7 instructions a pass, as the count is tested, stored and taken
  down, then the work done. `skip_cards(n)` and `add_times(c, n)` are a computed jump into a row of ten `INP 00` or
  nine `ADD c`. Each row is a routine put in once (27 and 34 cells), and costs about 1.2 instructions a card and 1
  an addition, plus 17 and 27 a call. For a few passes (n below 4 or so) the loop is cheaper.
- `let x = if c { e } else { K }` in straight-line code: each arm sets x and one jumps past the other, about 3 cells
  more than `let mut x = K; if c { x = e; }`, whose K is x's starting value in the deck when it runs once. The
  compiler doesn't do this itself, because storing K first keeps x alive across the test, which can cost a shared
  cell more than it saves. Write `let mut x = K; if c { x = e; }` in code that runs once.

**Refused**

- `let Some(x) = o;` (Rust's own rule): rustc refuses it (E0005), so it builds nowhere. Use `let else` or `if let`.

### [Data](manual/types.md#arrays)

**Works well**

- Fixed-size arrays of any of the types above, of structs, of tuples and of arrays. `[[i16; 3]; 3]` is one array,
  laid out by tens where the program still fits that way: each row starts ten cells after the one before, and the
  program's other cells go in the holes at the ends of the rows. Otherwise it is row after row.
- `static` and `static mut`.
- Tuples, nested too, passed and returned.
- Structs with named fields, tuple structs, unit structs and generic structs, with methods (`impl`), functional
  update and destructuring.
- A `union` read through the field it was written by.
- Shadowing and block expressions.

**Costs more than it looks**

- An array or pointer read through a variable index: self-modifying code, since CARDIAC has no indexed addressing.
  The instruction is built from the address, stored and run, a few cells and steps each time. An array of arrays
  costs a few instructions more an index: by tens the row is one decimal shift (`t[10 * r + c]`), and row after
  row it is r added W times, as `t[r * W + c]` by hand does. `t[r + 1][c + 2]` is found from `10 * r + c`, with
  the 12 in the instruction's address, unless r or c may be a usize past a card (`inp() as usize` without overflow
  checks). Hoist a repeated `t[i]` into a local.
- Arrays of structs: one array per field, and an element passed whole copies each field. Parallel arrays, one per
  field, are cheaper.

**Refused**

- A row of an array of arrays borrowed (`&t[r]`, `t[r].iter()`) or copied out (`let row = t[1]`) (not done yet). Use
  `t[r][c]` one element at a time.

### [C-like enums](manual/items.md#enumerations)

**Works well**

- `enum Colour { Red, Green, Blue }`, with numbers given (`Low = -5`, `High = 3 * 300`) or not, and `#[repr(i16)]`
  or `#[repr(u8)]`. One cell, the variant's number.
- `match`, `as i16`, passed and returned.
- `==`, `<` and `cmp` of a derived `PartialEq`, `PartialOrd` or `Ord`.

### [Enums with data](manual/types.md#enums)

**Works well**

- `enum Shape { Dot, Circle(i16), Rect { w: i16, h: i16 } }`, generic enums of the program's own, `Option` and
  `Result`: built, matched, `if let`, passed and returned, nested.
- `?`, also converting the error with `From`.
- `Option`'s and `Result`'s methods: `is_some`, `unwrap`, `expect`, `unwrap_or`, `unwrap_or_else`,
  `unwrap_or_default`, `map`, `map_or`, `and_then`, `filter`, `ok_or`, `or`, `xor`, `zip`, `take`, `replace`,
  `insert`, `get_or_insert`, `as_ref`, `as_mut`, `copied`, `map_err`, `ok`, `err` and the rest.
- `==` and `<` of two `Option`s.
- `unwrap()` and `expect()` panic with Rust's message (`Result`'s without the `Err`'s value).
- `Option<NonZeroI16>`, `Option<bool>` and `Option<&i16>` use their niche, as Rust does.

**Costs more than it looks**

- A tag cell, the variant's number, beside cells for the fields, which the variants share. Setting a variant stores
  the tag, and a `match` tests it. An `Option<i16>` whose payload the range pass can prove is never negative (a
  count from 0, an index, a card the program `assume()`s is 0 or more) has no tag: `None` is -1 in the payload's
  cell, as a hand-written sentinel is, so `is_none()` is one TAC and `unwrap_or(-1)` is the cell itself. Within one
  function rustc usually takes an `Option` apart itself. At opt-level 1, where rustc calls the methods, it costs a
  few cells more than at opt-level 2. `assume()` what the deck guarantees (`assume(c >= 0)`), so the niche can be
  proven.
- `==` of two `Option`s tests both variants and then the payloads.

**Refused**

- An enum with data in a struct that is itself in an enum (`Option<Card>`, with `Card` holding an `Option`) (not
  done yet). Pass the enum by value, or use a struct of the fields and a tag.

### [Functions](manual/items.md#functions)

**Works well**

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
- An `extern "C" fn` defined in Rust.

**Costs more than it looks**

- Recursion other than a call of itself at the end: a stack of its own, about 30 cells of code for each place the
  function calls itself, and a cell a call for each local still read after the call. The stack is as deep as the
  memory left over holds, and a call with no room panics with `stack overflow`. Fibonacci's two calls don't fit in
  96 cells. Use a loop, or a call of itself as the last thing it does, which is a loop.
- An array passed by value: copied element by element at every call, two instructions an element, and the
  parameter's own cells. Pass `&[T; N]`.
- A struct passed or returned by value: each field copied. That is cheap for a few fields, so small structs are fine
  as they are.
- An inlined helper called in many places: its code is copied into each. `#[inline(never)]` keeps one copy.

**Refused**

- An `extern "C"` block (CARDIAC can't): there is nothing on the other side of the call. Write a Rust function.
- A recursive function with a reference or a pointer among its locals or parameters, one that returns a struct, or
  one that keeps a local other than a number or a bool across the call; and recursion through more than one
  function, unless every call is at its end (not done yet). Use a loop, or pass values in and return them.

### [Closures and indirect calls](manual/statements-and-expressions.md#closures)

**Works well**

- Closures that capture or not, `move` closures and `FnMut` ones: called where they are made, passed to a generic
  function, returned as `impl Fn`, stored in a struct, or behind a `&dyn Fn`.
- Function pointers, chosen at run time, kept in an array, passed and returned.
- `&dyn Trait` of the program's structs, chosen at run time.

**Costs more than it looks**

- A call through a function pointer or a `&dyn Trait`: a test for each function it can be, then a direct call,
  about two cells a function. A `&dyn` is a tag beside its types' fields, and passing one passes them all. A direct
  call or a generic function is cheaper.

**Refused**

- An object chosen at run time whose methods multiply, with overflow checks on (too big: 117 cells, or 89 without
  them). Build with overflow checks off, or use a `match` on an enum.
- A closure called through `FnOnce`, or one that captures a field of a struct; a closure that captures nothing,
  behind `&dyn Fn`; `&mut dyn`, `Box<dyn ..>`, an object of a primitive or a generic struct, and a trait's default
  method called through an object; a function pointer compared, or kept in a struct, a tuple or an `Option` (not
  done yet). Use generics, a function pointer, or a `match` on an enum.

### [References](manual/types.md#references-and-pointers)

**Works well**

- `&mut` and `&` to a scalar, a struct or an enum.
- Arrays by reference, `&[i16; N]` and `&mut [i16; N]`.
- Slices, `&[T]` and `&mut [T]`, of a length known only at run time, subslices (`&t[1..n]`) and slice patterns
  (`[first, .., last]`).
- A reference to a reference, and a reference returned from a function.
- Raw pointers and `p.add(i)`.

**Costs more than it looks**

- A slice of run-time length: its length is in a cell of its own, passed at every call beside the address. Where
  the length is known, use `&[T; N]`.
- A struct or an enum behind a `&mut`: a `&mut` of each of its cells. The call passes an address a cell, and each
  read or write through one is self-modifying code, a few cells and steps. For a function that touches every field,
  pass the value in and return it.

**Refused**

- A struct with an array field behind a `&mut` (not done yet). Pass the struct in and return it.

### [Iterators](manual/library.md#iterators)

**Works well**

- On a range or an array's iterator: `sum`, `count`, `fold`, `max`, `min`, `find`, `position`, `all`, `any`, `last`
  and `nth`.
- Adaptor chains of `map`, `filter`, `enumerate`, `skip`, `take`, `rev`, `chain`, `zip` and `step_by`, from a range,
  an array, `once`, `repeat` or `successors`, consumed by `sum`, `count`, `fold`, `for_each`, `max` or `min`. A chain
  is compiled as the one loop it stands for.

**Refused**

- `filter_map`, `flat_map`, `peekable`, `scan` and `take_while`; `any`, `all`, `find` and `position` on a chain of
  adaptors; a chain kept and stepped by hand, or in a `for` loop; and `rev` of `enumerate`, `skip`, `take`, `zip`,
  `step_by` or `successors` (not done yet). Use a `for` loop over a range, or the method on a range or a plain array
  iterator.

### [Array and slice methods](manual/library.md#arrays-and-slices)

**Works well**

- `len`, `is_empty`, `get`, `first`, `last` and `contains`.
- `fill`, `swap`, `reverse`, `rotate_left`, `sort_unstable` and `binary_search`.
- `split_at`, `copy_from_slice`, `chunks` and `windows`.
- `[T; N]::map` and `core::array::from_fn`.
- `==` of two arrays.

### [Strings and text](manual/library.md#strings)

**Works well**

- A string literal's `len()`, its bytes (`as_bytes()`) and indexing. A literal is a cell a byte.
- Byte strings (`b"CARD"`).
- `concat!`, `stringify!`, `file!` and `line!`.

**Refused**

- `println!`, `print!`, `format_args!` and `write!` (CARDIAC can't): CARDIAC prints only numbers. Use `out()`.
- Comparing strings (CARDIAC can't): two short literals compared come to about 124 cells. Use numbers in place of
  words.
- Parsing them, as `i16::from_str_radix` does (CARDIAC can't): a program's input is numbers on cards.

### [The rest of core](manual/library.md#the-rest-of-core)

**Works well**

- `core::mem::swap`, `replace` and `take`.
- `Ordering` and its methods, `then`, `partial_cmp` and `core::cmp::Reverse`.
- `bool::then` and `then_some`.
- `char::to_digit`, `char::from_digit` and `is_ascii_digit`.
- `core::hint::black_box`, `assert_unchecked` and `unreachable_unchecked`.

### [Checks](manual/panic.md)

**Works well**

- `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!` and its kin.
- `panic!()`, `unreachable!()`, `todo!()` and `unimplemented!()`, with a literal message or a formatted one. The
  panic map keeps the literal text, without the values.
- Bounds and overflow checks, and `assume()`.
- A check rustc has proved fails (`#[allow(unconditional_panic)]`), which is the panic.

**Costs more than it looks**

- Overflow checks on, as in `dev`: each checked `i16` sum calls a shared check routine, and every checked operation
  is a test and a branch. An iterator's `sum` is checked whatever the build's setting, as core's is. Build with
  `--release` once the program is right.

### [Macros and attributes](manual/macros.md)

**Works well**

- `macro_rules!`, `matches!`, `cfg!` and `#[cfg]`.
- Derives of `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`, and `Debug` on a value
  that is never formatted.
- `#[repr]`, `#[inline]`, `#[cold]` and `#[track_caller]`.
- `#[test]` functions beside the program.

**Refused**

- `compile_error!` (Rust's own rule): it builds nowhere.

### [Memory](manual/memory.md)

**Works well**

- A program bigger than memory, in phases: `overlay()` in `main` where a phase ends, or `auto-overlay` on.

**Costs more than it looks**

- `opt-level = 0`: rustc's unoptimised MIR, and the compiler's own passes off, so most programs don't fit in 96
  cells. Use opt-level 1 or more (cargo's `dev` profile is fine).

**Refused**

- `Box`, `Vec`, `String`, `vec!`, `format!`, and a recursive enum through `Box` (CARDIAC can't): every value's cell
  is fixed when the program is built. Use fixed-size arrays.

### [Built-ins](manual/runtime.md)

**Works well**

- `inp()` and `out()`.
- `overlay()`, for a program in phases.
- `load_resource("NAME")`, for data loaded with the deck.
- `digit()`, `skip_cards(n)` and `add_times(c, n)`.

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
