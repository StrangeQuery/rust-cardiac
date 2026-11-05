# The Rust that rust-cardiac compiles

What compiles for CARDIAC and what doesn't, in three groups: what works well, what works but costs more than it
looks, and what is refused. Every entry was checked by compiling a small program at opt-level 2 with overflow
checks on (cargo's `dev` profile) and running it both ways: on the CARDIAC emulator and natively with `cargo run`.
"Works" means the two print the same; a refusal is a compile error, never a wrong answer.

## Who checks what

**rustc does the whole front end:** parsing, type checking, trait resolution, the borrow checker, constant
evaluation. A program rustc rejects never reaches CARDIAC, and its errors are Rust's own
(`tests/programs/borrow_error.rs` stops at E0499). rust-cardiac compiles the MIR rustc writes for the program
that's left: so anything rustc folds away first (a `const fn`, a closure called where it's made, a small generic
function it inlines) costs nothing and needs no support.

**Bounds checks are always on**, as in Rust. **Overflow checks follow the profile** (`overflow-checks`, on in
`dev`, off in `release`). A failed check outputs -999 and halts; with the panic map beside the deck, burn then says
why, as a Rust panic does, and exits 101 (`panicked in main: index out of bounds`). `unsafe { assume(cond) }` (Rust's `assert_unchecked`)
lets the compiler leave out a check it can then prove; break the assumption and the program reads whatever is there.
With debug assertions on (`debug-assertions` in the profile) the assumption is checked on CARDIAC too, and natively
it always is. So are the standard library's own checks of unsafe preconditions that debug assertions keep
(`unchecked_add` and its kin, `assert_unchecked`, `unreachable_unchecked`): a violated one outputs -999 and halts,
and burn exits 134, as Rust aborts (`unsafe precondition(s) violated: usize::unchecked_sub cannot
overflow`). The ones a `for` loop's own test proves cost nothing.

**One limit is CARDIAC's, not Rust's:** a cell holds -999..=999. An `i16` past that panics on CARDIAC with overflow
checks on, where native Rust carries on to 32767; with them off, CARDIAC keeps the low three digits (1100 is 100)
where Rust would keep the whole value. A program whose values fit on a card prints the same both ways. A `u8`
wraps as Rust's does (`0u8 - 1` is 255). A `usize` past a card, as `-1i16 as usize` makes one (usize::MAX), or
without overflow checks `0usize - 1`, is held as the negative number it is short of 2^64 by: it compares above
every other `usize`, casts back to `i16`, or to `u8`, as in Rust, can be added to and subtracted from, and as an
index fails the bounds check; anything else done with it (passed to a function, multiplied, divided, an
overflow-checked `+` or `-`) panics there, as a value past a card does.

## 1. Works well

Use these freely: they compile to what you'd write by hand.

| Area | What works |
|------|------------|
| **Types** | `i16`, the card's own number; `usize` for indexes and counters; `bool`; `u8`. An integer literal with no type (`i32` by default) works as an `i16` would. |
| **Arithmetic** | `+`, `-`, unary `-`, comparisons. `*` by a constant (adds and decimal shifts), `/` and `%` by a power of ten (shifts). `digit(v, k)` for one digit of a number; `add_times(c, n)` for `n * c` by additions (n 0..=99). `x & m` for a mask m one below a power of two (`x & 3` is `x.rem_euclid(4)`, a subtracting loop). |
| **Integer methods** | On `i16`: `.abs()`, `.wrapping_abs()`, `.is_negative()`, `.is_positive()`, `.wrapping_add()`, `.wrapping_sub()`, `.wrapping_mul()`, `.wrapping_neg()`, `.rem_euclid()`, `.div_euclid()` by a constant; `.signum()` at opt-level 1 and s. On `i16`, `u8` and `usize`: `.min()`, `.max()`, `.clamp()`, and `core::cmp::min` and `max`. On `u8` and `usize`: `.saturating_sub()`, `.abs_diff()` (a `u8`'s at opt-level 1 and s). Where rustc calls one rather than inlining it (opt-level 1 and s, and `min`, `max`, `clamp` at every level), the compiler writes the code rustc would have inlined: a test or two, and `0 - x` for a negation. No card holds `i16::MIN`, so `abs` has nothing to check; `clamp` checks `min <= max`, and `rem_euclid` a divisor of zero, as Rust does. |
| **Control flow** | `if`/`else`, also as an expression; `while`; `loop`, also with `break value`; labelled `break` and `continue`; early `return`; `match` on integers, with ranges, guards and tuples of them; `for` over a range (`a..b`, `a..=b`, `.rev()`). |
| **Data** | Fixed-size arrays of `i16`, `u8` or `bool`, indexed by anything; `const` values and tables, also from other crates; tuples in a function, and tuples of scalars passed to and returned from one; structs, whose fields are scalars, arrays of scalars, structs or enums, also as literals passed by value or by reference, with methods (`impl`); shadowing and block expressions. |
| **C-like enums** | `enum Colour { Red, Green, Blue }`, with numbers given (`Low = -5`, `High = 3 * 300`) or not: one cell, the variant's number; `match`, `as i16`, passed and returned: the MIR names a variant but not its number, which the CARDIAC driver looks up. `==` and `!=` of a `#[derive(PartialEq)]` enum, also where rustc calls its `eq` (opt-level 1 and s). |
| **Enums with data** | `enum Shape { Dot, Circle(i16), Rect(i16, i16) }`, `Option<i16>`, `Result<i16, i16>`: built, matched, `if let`, passed and returned, nested (what `?` builds). `?` within a function; on `Option` `.is_some()`, `.is_none()`, `.unwrap_or()`, `.unwrap()`, `.expect()`, `.ok_or()`, and on `Result` `.is_ok()`, `.is_err()`, `.unwrap_or()`, `.unwrap()`, `.expect()`, `.ok()`, `.err()`, also where rustc calls them (opt-level 1 and s); `==` and `!=` of two `Option`s; `checked_add`, `checked_sub` and `checked_mul` on an `i16`, `None` past a card (999), as every `i16` overflows there; `.map()` with a closure, on `Option` and `Result`: any closure where rustc inlines it (opt-level 2 and 3), and one that captures nothing where it calls it (1 and s). `unwrap()` and `expect()` panic with Rust's message (`Result`'s without the `Err`'s value). Fields are scalars (`i16`, `u8`, `usize`, `bool`), C-like enums, or enums with data. Costs a tag cell: see group 2. |
| **Functions** | Parameters and a result; small helpers rustc inlines (`#[inline(never)]` keeps one shared copy); traits with static dispatch; methods taking `self` by value, inherent or a trait's, also where rustc calls them rather than inlining them; `const fn`; closures called where they are made; generic functions rustc inlines. No recursion: see group 3. |
| **References** | `&mut` to a scalar (`fn bump(v: &mut i16)`), or to a struct or an enum (`fn step(o: &mut Option<i16>)`); a shared `&` to a scalar or a struct (`fn show(p: &Pair)`), passed as the value itself where every reference it meets can be; arrays by reference, `&[i16; N]` and `&mut [i16; N]`; raw pointers. |
| **Checks** | `assert!`, `panic!()`, `unreachable!()`, bounds and overflow checks, `assume()`; a check rustc has proved fails (`#[allow(unconditional_panic)]`) is the panic. |
| **Built-ins** | `inp()`, `out()`, `overlay()` for a program in phases, `load_resource("NAME")` for data loaded with the deck, `digit()`, `skip_cards(n)` and `add_times(c, n)`. See the README. |

## 2. Works, but costs more than it looks

These compile and give the right answer. Each has a cheaper way, when the cells or the time matter.

| Construct | What it costs | Cheaper |
|-----------|---------------|---------|
| `*` of two variables | A multiply routine, put in once (about 35 cells), and repeated addition: as many steps as the smaller factor. | Multiply by a constant; keep the smaller factor second. |
| `/` or `%` of two variables | A divide routine, put in once (about 40 cells for `/`, 50 for `%`), and repeated subtraction: as many steps as the quotient. | Divide by a power of ten; `digit()` for one digit. |
| A loop of `inp()`s or of `+= c`, n times | 6 or 7 instructions a pass: the count tested, stored and taken down, then the work. | `skip_cards(n)` and `add_times(c, n)`: a computed jump into a row of ten `INP 00` or nine `ADD c`, a routine put in once (27 and 34 cells), about 1.2 instructions a card and 1 an addition, plus 17 and 27 a call. For a few passes (n below 4 or so) the loop is cheaper. |
| An array passed by value | Copied element by element at every call: two instructions an element, and the parameter's own cells. | Pass `&[T; N]`. |
| A struct passed or returned by value | Each field copied. Cheap for a few fields. | Fine as it is, for small structs. |
| `==`, `!=`, and a `match` on values | CARDIAC's one test is TAC, "jump if negative": `x < k` is a subtraction and a TAC, but `x == k` is two (not below, not above), about six instructions. A `match` tests its arms one after another, each an equality, and a chain of `else if x == k`; a value tested once and then again (`let b = if c { 1 } else { 0 }; if b == 1`) costs both, unless the compiler can follow the constant straight to the arm. `==` of two `Option`s tests both variants and then the payloads. | Test with `<` and `>=` where the values allow (`if x < 0`, ranges in a `match`), the most common arm first; keep a value's sign meaningful (-1 for none: the niche layout does this for `Option<i16>`). |
| An enum with data | A tag cell, the variant's number, beside cells for the fields, which the variants share; setting a variant stores the tag, a `match` tests it. An `Option<i16>` whose payload the range pass can prove is never negative (a count from 0, an index, a card the program `assume()`s is 0 or more) has no tag: `None` is -1 in the payload's cell, as a hand-written sentinel is, so `is_none()` is one TAC and `unwrap_or(-1)` is the cell itself. Within one function rustc usually takes an `Option` apart itself. At opt-level 1 and s, where rustc calls the methods, a few cells more than at opt-level 2. | `assume()` what the deck guarantees (`assume(c >= 0)`), so the niche can be proven. |
| `.rem_euclid()` by a variable | The divide routine, as for `%`, and about 10 cells more to make a negative remainder positive. | By a constant: a power of ten is shifts, as for `%`. |
| An array or pointer read through a variable index | Self-modifying code: the instruction is built from the address, stored and run, a few cells and steps each time. | Unavoidable on CARDIAC (no indexed addressing); hoist a repeated `t[i]` into a local. |
| Overflow checks on (`dev`) | Each checked `i16` sum calls a shared check routine, and every checked operation is a test and a branch. | Build with `--release` once the program is right. |
| Arrays of structs | One array per field, and an element passed whole copies each field. Only when the program builds a value of that struct type at run time somewhere (a function returning one, say): otherwise the compiler hasn't learned its fields, and refuses it (a gap, to be fixed). | Parallel arrays, one per field. |
| A struct or an enum behind a `&mut` | A `&mut` of each of its cells: the call passes an address a cell, and each read or write through one is self-modifying code, a few cells and steps. | The value passed in and returned, for a function that touches every field. |
| `let x = if c { e } else { K }` in straight-line code | Each arm sets x and one jumps past the other: about 3 cells more than `let mut x = K; if c { x = e; }`, whose K is x's initial value in the deck when it runs once. The compiler doesn't do this itself: storing K first keeps x alive across the test, which can cost a shared cell more than it saves. | `let mut x = K; if c { x = e; }`, in code that runs once. |
| An inlined helper called in many places | Its code copied into each. | `#[inline(never)]` keeps one copy. |
| `opt-level = 0` | rustc's unoptimised MIR, and the compiler's own passes off: most programs don't fit in 96 cells. | opt-level 1 or more (cargo's `dev` profile is fine). |

## 3. Refused: these don't compile

The build stops with an error. For the common ones the error says what to do instead, in Rust's terms.

| Area | Refused | Instead |
|------|---------|---------|
| **Constants** | A constant past what a card holds (-999 to 999): a literal like `1010`, or arithmetic rustc works out at compile time (`101 * 10`), even in a branch no run takes. It is Rust's own rule, for a card instead of an `i16`: rustc refuses `30000 * 2` in an `i16` (`arithmetic_overflow`, an error by default) and `i16::MAX + 1` in a `const`. Like rustc's, it is as good as rustc's folding: `x * 10` with x known to be 101 may be refused at one opt-level and panic as an overflow at run time at another. | Keep constants on a card; compute what may pass one at run time, where the overflow check sees it. |
| **Types** | `i8`, `u16`, `u32`, `i64`, `u64`, `i128`, `f32`, `f64`; `String`, `&str` beyond a literal, `Vec`, `Box`, anything on the heap. | `i16`, `u8`, `usize`, `bool`. |
| **Casts** | `i16 as u8` (it wraps). `t[i] as i16` straight from a `u8` or `bool` array. | Compare and clamp; copy the element to a local, then cast. |
| **Bit operations** | `<<`, `>>`, `\|`, `^`, `!` on integers; `&` but by a mask one below a power of two. | `*` and `/` by powers of ten and two; `digit()`. (`!` on a `bool` works.) |
| **Enums** | An enum with data in a struct that is itself in an enum (`Option<Card>`, `Card` holding an `Option`); at opt-level 1 and s, `Option`'s methods that take `&mut self` (`.take()`), and `.map()` with a closure that captures a variable. | The enum by value; a struct of the fields and a tag; `match`; `if let`; plain `+` and a range check. |
| **Library methods** | `.pow()`, `.div_euclid()` by a variable (by a constant it works); at opt-level 2 and 3, `.signum()` and a `u8`'s `.abs_diff()` (rustc inlines them into a three-way compare, `Cmp`, and a subtraction in `i32`); `i16`'s `.saturating_add()` and its kin (they saturate at 32767, which no card holds), `.abs_diff()` and `.unsigned_abs()` (a `u16`); wrapping and saturating arithmetic on `u8` and `usize` other than `.saturating_sub()`; `core::mem::swap`, iterator adaptors. | A loop of `*`; `/` with `.rem_euclid()`; `if x < 0 { -1 } else if x > 0 { 1 } else { 0 }`; plain `+`; a temporary. |
| **Data** | Arrays of `usize`, arrays of arrays (`[[i16; 3]; 3]`), arrays of tuples, slices (`&[i16]`), `static` and `static mut`. A struct with an array field behind a `&mut` or in an array of structs. | An `i16` array; one array, indexed `row * width + column`; `&[T; N]`; a local in `main` passed down. |
| **Iteration** | Iterators other than ranges: `.iter()`, `for x in array`, `.step_by()`, `.sum()`, `.map()`. | `for i in 0..N` and `t[i]`; a `while` loop for steps other than 1. |
| **Functions** | Recursion (each function has fixed cells: there is no stack). A generic function rustc doesn't inline (`#[inline(never)]`, or from another crate). A tuple of other than scalars passed or returned. A closure passed to a function (but to `Option`'s and `Result`'s `map`). | A loop; a non-generic function, or `#[inline]`; a struct for the result; the struct by value, returned; flat structs. |
| **Macros** | `println!`, `format!`, `assert_eq!`, `assert_ne!`. | `out()`; `assert!(a == b)`. |

## How this was checked

About 90 small programs, one feature each, compiled and run as described at the top; the test programs in
`tests/programs` and the examples cover the same ground at more length (`./test_compiler.sh`, `./test_examples.sh`).
Probing this way found two miscompiles, both fixed: a loop whose condition starts with a call that isn't inlined
ran its body once (jump threading went through the call), and an array passed by value got only its first element.
