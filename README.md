<p align="center"><img src="https://raw.githubusercontent.com/StrangeQuery/rust-cardiac/HEAD/assets/logo.png" alt="rust-cardiac: Ferris, the Rust crab, in front of a CARDIAC" width="320"></p>
<p align="center"><a href="https://lab.strangequery.com/rust-cardiac/"><code>lab.strangequery.com/rust-cardiac</code></a></p>
<p align="center"><a href="https://github.com/StrangeQuery/rust-cardiac"><img src="https://img.shields.io/badge/GitHub-rust--cardiac-181717?logo=github&amp;logoColor=white" alt="GitHub"></a> <a href="https://discord.gg/VsqKEdBYzJ"><img src="https://img.shields.io/badge/Discord-join-5865F2?logo=discord&amp;logoColor=white" alt="Discord"></a></p>

# rust-cardiac

The newest member of the [Strange Query CARDIAC](https://lab.strangequery.com/cardiac/) collection, [rust-cardiac](https://lab.strangequery.com/rust-cardiac) brings the convenience, safety and security of Rust to the most legacy of legacy platforms: the [CARDboard Illustrative Aid to Computation](https://en.wikipedia.org/wiki/CARDboard_Illustrative_Aid_to_Computation).  Compile Rust to either the classic six-column
listing form or raw cards ready to be fed directly into a CARDIAC and executed.  For demos and the most up-to-date information see the
[rust-cardiac home page on Strange Query](https://lab.strangequery.com/rust-cardiac/).  Open Source and MIT-licensed.

rust-cardiac is two crates. [cargo-cardiac](https://crates.io/crates/cargo-cardiac) is what you install: the `cargo
cardiac` command, which makes, builds, runs and debugs CARDIAC programs. [`cardiac`](https://crates.io/crates/cardiac)
is what your program uses: `inp()`, `out()` and CARDIAC's other built-ins, which `cargo cardiac new` adds to your
Cargo.toml for you.

## Part of a full development platform
rust-cardiac integrates with [Cardigan](https://lab.strangequery.com/cardigan/) and [heartburn](https://github.com/StrangeQuery/heartburn)
to give you a full development environment and high-performance runtime. Build and run Rust with `cargo cardiac run`, which runs it _fast_ on heartburn's burn, and step through it with `cargo cardiac debug` or in [Cardigan](https://lab.strangequery.com/cardigan/).

## Quick start
```
cargo install cargo-cardiac
cargo cardiac setup
cargo cardiac new countdown
cd countdown
cargo cardiac run
```

`cargo cardiac setup` downloads what the compiler needs, once: its driver (about 1 MB) and the Rust nightly the
driver runs on (about 550 MB on disk). Skip it and the first build does the same.

`cargo cardiac new` makes a package as `cargo new` does, with `cardiac` as a dependency and a `src/main.rs` that
counts down from the number it reads. `cargo cardiac run` builds it for CARDIAC and runs it. Type a number and
press Enter, and it counts down to 0. Or give it the number on standard input, `echo 5 | cargo cardiac run`, or
after `--`, `cargo cardiac run -- 5`.

The rest is on [lab.strangequery.com/rust-cardiac](https://lab.strangequery.com/rust-cardiac/): the listing to
open in Cardigan, panic maps, running a deck from your own Rust program, and what Rust works.

## What's new

**0.2.4.** A clearer opening for cargo-cardiac's docs.

**0.2.3.** Tweaks to the compiler's messages.

**0.2.2.** Three wrong answers fixed, all with values past a card. A `u8` or `i8` multiply past a card now
compiles without overflow checks. An internal compiler error asks for a bug report. New:
[the rust-cardiac reference guide](https://lab.strangequery.com/rust-cardiac/manual/).

**0.2.1.** New crate descriptions.

**0.2.0.** Two wrong answers in 0.1.3 are fixed. A value borrowed for a call could share a cell with another value
and be read wrongly, and an overflow-checked sum of three terms could lose its first term when another of them was
an array element at an index worked out at run time.

Much more Rust compiles: slices of run-time length and their methods, iterator adaptors (`map`, `filter`, `zip`,
`chain`, `skip`, `take` and the rest), function pointers and `dyn Trait`, closures passed, returned and stored,
recursion (a function that calls itself gets a stack of its own, and functions that call one another at their
ends need none), the bitwise operators and the bit methods, `u16` and `isize`, panic messages with values in them,
nested tuples and arrays of arrays, the integer methods (`pow`, `isqrt`, the saturating ones and the rest), and a
string literal's length and bytes.

Every message about a program now has a code: RCE for an error, RCW for a warning and RCI for a note. `cargo
cardiac help CODE` says what one means, and so does its page in
[the message codes](https://lab.strangequery.com/rust-cardiac/messages/). A program that uses an expensive
operation, such as `&` between two variables or `count_ones()`, gets a warning that names it. `--edition` sets a
single file's edition. And `cargo cardiac` says when a newer version is out, at most once a day
(`CARGO_CARDIAC_NO_UPDATE_CHECK=1` turns that off).

**0.1.3.** `cargo cardiac view` builds a program and opens it in Cardigan. Slices of arrays and their iterators,
`==` of arrays, `step_by`, and generic functions rustc doesn't inline compile.

**0.1.2.** One install for everything: `cargo cardiac new`, `setup`, `run`, `debug` and `asm`. `i8`, `char`,
statics, unit structs, `Ordering`, `assert_eq!` and panics with a message compile, and some wrong answers are fixed.

**0.1.1.** Shorter crate descriptions, and heartburn from crates.io.

**0.1.0.** The first release.

## CARDIAC

CARDIAC (CARDboard Illustrative Aid to Computation) was a paper computer that Bell Labs gave to schools in 1968.
It has 100 memory cells of three decimal digits and a sign, an accumulator, ten instructions (read a card,
load, add, subtract, store, shift, test for negative, jump, output, halt), a card reader for input and a teletype
for output. See [The Strange Query CARDIAC Archive](https://lab.strangequery.com/cardiac/) for more information on CARDIAC, its
history and lore, samples, the machine specification, the opcodes, and programming tips and tricks.

<p align="center"><img src="https://raw.githubusercontent.com/StrangeQuery/rust-cardiac/HEAD/assets/cardiac.jpg" alt="A CARDIAC: the cardboard computer, its slides pulled out" width="360"></p>

## Getting started

You need rustup (https://rustup.rs, not a distribution's packaged Rust). Install cargo-cardiac:

    cargo install cargo-cardiac

The compiler runs inside rustc. Its backend is the CARDIAC driver, which is rustc built as a library on one
pinned nightly, `nightly-2026-05-22`, the last of Rust 1.97's. The first time you build a program, `cargo cardiac`
downloads the driver for your platform from this version's GitHub release (about 1 MB). If rustup doesn't
have that nightly yet, it also asks rustup to install it, in its minimal profile: about 550 MB on disk, once.
`cargo cardiac setup` does both now rather than at the first build, and says when there is nothing left to do. Your
own Rust stays as it is. Programs can use what Rust 1.97 has. To build the compiler from source instead, see
[CONTRIBUTING.md](https://github.com/StrangeQuery/rust-cardiac/blob/HEAD/CONTRIBUTING.md).

## Hello
`examples/hello.rs` counts down from ten to zero.

```rust
//! Hello, world. CARDIAC's teletype prints numbers, so this one counts down: 10, 9, ... 1, 0.
use cardiac::*;

fn main() {
    for n in (0..=10).rev() {
        out(n);
    }
}
```

Notice the use of `out()` instead of `println!`. CARDIAC has limited I/O. `println!` goes through Rust's formatting machinery, `core::fmt`. That's thousands of instructions of code, and CARDIAC has 96 cells. `out()` is a built-in the compiler recognises by name and turns into a single OUT instruction. CARDIAC can only print a number per line anyway, so `out()` covers everything `println!` could do there.

When you run the program natively, `out()` literally is `println!`. In the `cardiac` crate ([`cardiac/src/lib.rs`](https://github.com/StrangeQuery/rust-cardiac/blob/HEAD/cardiac/src/lib.rs)), native `out()` calls `println!("{v}")`, after checking that the value fits on a card, -999 to 999. Similarly, `inp()` stands in for reading a line from stdin and parsing it as a number. Natively that's `std::io::stdin().lock().read_line(...)` plus a parse, and it stops the program when the input runs out, as an empty card reader would.

The [rust-cardiac reference guide](https://lab.strangequery.com/rust-cardiac/manual/) has the list of what is and isn't supported. `println!` and `format!` are refused: `out()` prints a value. `assert_eq!`, `assert_ne!` and panics with a message work, and say what Rust's would, without the values a formatted message prints.

## Building and running Hello
In a checkout of this repository, in `examples/`, run it natively, then on CARDIAC:

    cargo run --bin hello                # 10, 9, ... 0, native code
    cargo cardiac run --bin hello        # the same 10, 9, ... 0, on CARDIAC
    cargo cardiac build --bin hello      # only build the CARDIAC deck, hello.cqb

`cargo cardiac build` writes `hello.cqb`, the deck, and `hello.map`, its panic map, next to `hello.rs`. `cargo
cardiac run` builds them the same way and then runs the deck. Leave out `--bin` to build every program in the
package (`run` runs one, so a package of several needs it). To watch it run, open Cardigan and drop `hello.cqb` on it. To see the Rust beside the CARDIAC code, build with
`--emit deck,listing,map`, then drop `hello.cqb`, `hello.cq` and `hello.map` on Cardigan together. It shows the
listing, and if the program panics, it says why.

## Your own program

`cargo cardiac new countdown` made a package with `cardiac` in its `[dependencies]` (`cargo add cardiac` does the
same for a package you already have) and this `src/main.rs`:

```rust
use cardiac::*;

fn main() {
    let mut n = inp();
    while n >= 0 {
        out(n);
        n -= 1;
    }
}
```

    echo 5 | cargo run                  # 5 4 3 2 1 0
    echo 5 | cargo cardiac run          # the same, on CARDIAC; or: cargo cardiac run -- 5
    cargo cardiac build                 # only build: src/countdown.cqb and src/countdown.map

Like `cargo build`, `cargo cardiac build` uses the `dev` profile, `--release` the `release` one, and
`--profile NAME` one of your own. cargo's `dev` profile doesn't suit CARDIAC, so for a dev build `cargo cardiac`
changes five of its settings, and says so each time it builds:

```toml
[profile.dev]
opt-level = 2              # CARDIAC needs rustc's optimized MIR: at opt-level 0 most programs don't compile
incremental = false        # the CARDIAC driver can't build incrementally
debug = false              # debuginfo changes the MIR, and with it the listing
overflow-checks = false    # the checks' code rarely fits in 96 cells
debug-assertions = false
```

It changes only what your Cargo.toml doesn't set, in `[profile.dev]` or for the package. Set any of these yourself
and yours is used, and the note stops mentioning it. `cargo run` keeps cargo's own settings. That means an
overflow `cargo run` panics on goes unchecked on CARDIAC, unless you set `overflow-checks = true`.

## Examples

Starter programs are bundled in `examples/`. Each of them produces the same output natively and on CARDIAC (`./test_examples.sh` checks that), and each fits in
CARDIAC's 96 cells. All but `collatz` are binaries of the package `examples`, whose `dev` profile is optimized and has no overflow checks. `collatz` counts on its overflow checks, and a profile is per package, so it is a package
of its own, `examples/collatz/`, built with `-p collatz`.

| program     | what it does                                                                   | input            | cells        |
|-------------|--------------------------------------------------------------------------------|------------------|-------------:|
| `hello`     | counts down from 10 to 0                                                       |                  |  8           |
| `fibonacci` | the Fibonacci numbers up to 987, the last that fits on a card                  |                  | 19           |
| `fizzbuzz`  | FizzBuzz to 100: Fizz prints -3, Buzz -5, FizzBuzz -15                          |                  | 45           |
| `primes`    | the primes below 50, by the sieve of Eratosthenes                              |                  | 83           |
| `gcd`       | the greatest common divisor of two numbers, by Euclid's algorithm              | `252 105`        | 79           |
| `collatz`   | the Collatz sequence from a number, until it reaches 1 (27 overflows: -999)    | `7`              | 46           |
| `sort`      | reads ten numbers, prints them smallest first (selection sort)                 | ten numbers      | 71           |
| `powers`    | 2 to the power n, up to 2^99, as an array of digits; in two phases             | `99`             | 31 + 53 / 25 |
| `cipher`    | substitutes each digit of numbers 0..999 through a key, a resource            | numbers          | 53           |
| `days`      | the days in a month, from a table (13 is out of bounds: a panic, explained)    | `2`              | 31           |
| `days_assumed` | `days` for a deck that only holds 1..12: `assume()` drops the bounds check | `2`           | 21           |
| `runs`      | counts runs of positive cards, keeping three flags as `cardiac::Flag`s        | cards, then `-999` | 57         |
| `rot13`     | ROT13, letters as 1..26: the message is the deck behind the program           | letters          | 31           |
| `guess`     | guess the number: 1 to 100 from a random seed; each guess answered +1 or -1  | a seed, guesses  | 49           |
| `nim`       | the manual's single-pile Nim: take 1 to 3 of ten, never what was just taken   | who starts, moves | 96          |
| `map`       | MapReduce's map: counts the words (1..16) of a shard ended by -999; outputs a deck | words, `-999` | 81         |
| `reduce`    | MapReduce's reduce: adds up the map machines' decks, stacked behind their number | decks         | 91           |
| `voxel`     | a voxel landscape's projection and light, a depth slice at a time: `voxel.sh` draws a picture with it | a slice | 87 |
| `ray/`      | a ray tracer in ten machines, a mirror ball over a checkerboard: `ray.sh` draws a picture with them | `FRAME` | 45 to 91 each |



[voxel](https://lab.strangequery.com/rust-cardiac/demo/voxel/) and [ray](https://lab.strangequery.com/rust-cardiac/demo/ray/) both have extensive writeups on Strange Query's rust-cardiac home page.


## What Rust is supported

rustc does all of the front end: parsing, type checking, the borrow checker. A program it rejects never reaches CARDIAC, and you get Rust's own errors. cargo-cardiac compiles what rustc writes out. In short:

- **Works well:** `i16` (a card's -999..=999), `i8`, `u8`, `u16`, `usize`, `isize`, `bool`, `char` (one that fits on
  a card), and an `i32` whose values stay on a card; `+`, `-` and comparisons, `*` by a constant, `/` and `%` by a
  power of ten, `!`, and `<<` and `>>` by a constant; `if`, `while`, `loop`, `match`, `if let`, `let else` and `for`;
  arrays, arrays of arrays, slices, `const` tables, `static`s, tuples, and structs with methods; functions,
  generics, traits, `impl Trait`, closures, function pointers and `&dyn Trait`; references and raw pointers;
  iterators and their adaptors, the array and slice methods, and the integer methods; string literals' lengths and
  bytes; bounds and overflow checks, `assert!`, `assert_eq!`, `panic!` with a message.
- **Works, but costs more than it looks:** `*`, `/` and `%` of two variables (a multiply or divide routine, and as
  many steps as the numbers are big); `&`, `|` and `^` between integers (but a mask one below a power of two),
  shifts by a variable and the bit methods (routines that walk a card's bits, 26 to 67 instructions each, and the
  build warns about them);
  recursion other than a call to itself at the end (a stack of its own, about 30 cells, as deep as the memory left
  over holds); a call through a function pointer or a `&dyn` (a test for each function it can be); arrays passed by
  value (copied at every call); anything read through an index or a pointer (self-modifying code); arrays of
  structs (one array per field); overflow checks on.
- **Enums:** C-like ones are one cell. An enum with data, named fields or not, and so `Option` and `Result` with
  `?`, `if let` and most of their methods, is a tag cell and its fields' cells.
- **Refused, because CARDIAC can't do it:** integers wider than 16 bits (`i64`, `u32`, `u64`, `i128`, `u128`), since a
  cell holds -999 to 999; an `i32` is held as an `i16` is, so one past a card panics, or without overflow checks
  keeps its last three digits. Floats, since a cell holds whole numbers. The heap (`Box`, `Vec`, `String`,
  `format!`, `vec!`), since every value's cell is fixed when the program is built. Text formatting (`println!`,
  `write!`), since CARDIAC prints only numbers: `out()` prints a value. Comparing and parsing strings: a compare is
  too big, and the input is numbers, so there is no text to parse. `extern` blocks, since there is nothing outside
  to call. `swap_bytes`, whose result is past a card for almost every value. And a refutable `let` (`let Some(x) =
  o;`), which rustc refuses itself.
- **Too big for 96 cells:** a program that uses both `<<` and `>>` by a variable, and a `&dyn Trait` chosen at run
  time whose methods multiply, with overflow checks on.
- **Refused on purpose:** constants no card holds, such as `i16::MAX` and `i16::MIN` as values (comparing with one
  works), and `i16::midpoint`, which works in `i32` and would give a wrong answer past a card.

Anything else that is refused is work not done yet. The build stops with an error, and `cargo cardiac help CODE`
says what its code means and what to write instead.

The [rust-cardiac reference guide](https://lab.strangequery.com/rust-cardiac/manual/) has the whole list, each entry checked by running it both ways, with what each costs
and what to write instead. [tests/features/SUPPORT.md](https://github.com/StrangeQuery/rust-cardiac/blob/HEAD/tests/features/SUPPORT.md)
is the table behind it: one small program for each piece of Rust, in the Rust Reference's order, built four ways
and run on CARDIAC and natively.

## The built-ins

From the `cardiac` crate (`use cardiac::*;`):

| built-in                | on CARDIAC                                                                | with `cargo run`                            |
|-------------------------|---------------------------------------------------------------------------|---------------------------------------------|
| `inp() -> i16`          | INP: reads the next card                                                  | the next number on standard input           |
| `out(v: i16)`           | OUT: outputs a value                                                      | prints `v` on a line of its own             |
| `overlay()`             | HRS n: stops for deck n, the next phase, whose code loads over this one's | nothing                                     |
| `load_resource("NAME")` | an array the loader fills with the resource NAME: no code                 | `resources/NAME`, read when it runs         |
| `digit(v, k)`           | digit k of v (0 the hundreds, 1 the tens, 2 the units), by a shift        | the same, in arithmetic                     |
| `skip_cards(n)`         | reads n cards (0..=999) and drops them, by a jump into a row of `INP 00`  | n calls of `inp()`                          |
| `add_times(c, n)`       | `n * c` (n 0..=99), by a jump into a row of `ADD c`                       | `n * c`                                     |
| `assume(cond)` (unsafe) | `core::hint::assert_unchecked`: no code, so checks it proves can go       | `assert!`                                   |
| `Flag`                  | a `bool` kept as -1 (false) or 0 (true): a test is its sign               | a plain `i16`                               |

Natively, `inp()` takes numbers separated by spaces or new lines. When there are none left the program stops, as
CARDIAC stops when its reader is empty.

**Overflow.** With overflow checks on, arithmetic that leaves -999..=999 panics. With them off, a cell keeps the low
three digits and the program carries on. Natively, `out()` of a value no card holds panics, and every native panic
prints -999 first, the value CARDIAC's panic routine outputs. So a program whose values fit on a card prints the
same both ways.

**Phases.** A program bigger than memory comes in phases: `NAME.cqb`, then `NAME.2.cqb`, and so on. The data is
shared, and each phase's code is loaded over the one before. `overlay()` in `main` says where a phase ends, or the
compiler places the phases itself when the package turns `auto-overlay` on (`--auto-overlay` for a single file):

```toml
[package.metadata.cardiac]
auto-overlay = true
```

**Resources.** `load_resource("NAME")` is an array the program doesn't build but finds in memory: `let key: [i16;
10] = load_resource("key");`. Its values are the file `resources/NAME` beside the package's `Cargo.toml`, numbers
written as the input is. On CARDIAC the loader puts them in memory with the program, which costs no code. A resource
is loaded once, in `main` and outside any loop.

**skip_cards and add_times.** The cheapest way on CARDIAC to do one thing n times is a row of the same
instruction, entered by a jump the program builds itself, n cells from its end. `JMP 80` is the number 880, so
`CLA k880 / SUB n / STO next` makes the next cell `JMP (80 - n)`, and exactly n of the row run. A counted loop takes
6 or 7 instructions a pass, so for n below about 4 the loop is cheaper, and a constant n is cheaper still.

## Decks and panics

A deck (`NAME.cqb`) is plain text, one card per line, as a signed number (`002`, `-005`), nothing else. It starts
with two cards, 002 and 800, which turn cells 01 and 02 into a loader. Then come pairs, an address card and the value
for that cell, and it ends with the pair 002 8xx, a jump to the program's start. Cells that start at 0 are left out.
The program's input comes after the deck. Any CARDIAC that follows the 1968 manual can load it.

On CARDIAC a panic is the value -999 and a halt. `cargo cardiac build` also writes a panic map, `NAME.map`, beside
the deck, as a .pdb sits beside an .exe: where the program's checks are, and what each one checks. The deck is the
same with or without it. With the map beside the deck, `cargo cardiac run` says why a run panicked and exits 101,
as `cargo run` does:

    $ echo 13 | cargo cardiac run -q --bin days
    -999
         halted after 103 instructions
    cargo cardiac run: days.cqb: panicked in main: index out of bounds

Cardigan reads the map too, when it is opened with the deck.

## Where to run it

A .cqb file is an ordinary CARDIAC deck, so it isn't tied to our tools. These are the places we know it runs:

- **`cargo cardiac run`**, from the command line, which runs the deck on heartburn's burn. It's fast, it reads the
  panic map, and it loads the later decks of a program in phases (`NAME.2.cqb`, ...) when the program asks for
  them. burn is also a program of its own, for running decks without Rust: `brew install heartburn` (from
  Strange Query's tap, `brew tap strangequery/tap`), or the release archives of
  [heartburn](https://github.com/StrangeQuery/heartburn/releases).
- **[Cardigan](https://lab.strangequery.com/cardigan/)**, in the browser. Drop the deck on it to step through the
  program one instruction at a time, and drop the listing and the panic map with it to see the source and why a
  run panicked.
- **[Brian Stuart's simulator](https://www.cs.drexel.edu/~bls96/museum/cardsim.html)**, from his
  [CARDIAC page](https://www.cs.drexel.edu/~bls96/museum/cardiac.html) at Drexel. This is the best-known CARDIAC
  simulator, and the one we check against. Paste the deck into the Deck box with the program's input after it,
  then press Load, Reset and Run. Every example prints there exactly what it prints on burn and with `cargo run`.
  The exception is a program in phases: the simulator has no way to load a second deck, so only single-phase
  programs run there.
- **[Brent Wilkins's simulator](http://cardiac.wilkins.in/)**
  ([source](https://github.com/BrentWilkins/CARDIAC)). Paste the deck and the input into the Input box and press
  Run. Its SFT instruction has a bug: it uses a variable that doesn't exist, so the first shift stops the
  program. Programs that never shift run correctly. Of the examples, those are hello, fibonacci, fizzbuzz,
  primes, gcd, sort, days, runs, rot13 and nim. collatz, cipher, guess and voxel shift, so they stop there.
- **[CardiacVisualizer](https://github.com/fzaca/CardiacVisualizer)**, a desktop emulator in Rust. It has two
  bugs. ADD sets the sign flag from the number it adds instead of from the sum, so a TAC after an ADD can go the
  wrong way. And a TAC that jumps also writes cell 99, which only JMP should do, so it wipes out a subroutine's
  return address. Every example runs correctly except collatz and voxel, which hit them. With both fixed, all of
  them run.
- **[jawhitti/Cardiac](https://github.com/jawhitti/Cardiac)**, a 2016 interpreter and debugger in C#. Its SFT
  doesn't keep the accumulator to four digits, so digits that should drop off the top come back, and cipher and
  voxel go wrong. Its command line only takes inputs from 1 to 999, and it reads a path that starts with `/` as
  an option. To get around both, append the input cards to the deck file and pass the file by a relative path.
  When a program runs out of input it throws an exception, where CARDIAC simply stops. Every other example runs
  correctly.
- **[cinc](https://sourceforge.net/projects/cinc/)**, in Java (jcinc), with a window or a command line:
  `java -jar jcinc.jar -c -i DECK`, with the input after the deck in the same file. When the card reader is empty,
  INP reads -999 and the program carries on, where CARDIAC stops. So a program that reads until the cards run out
  never ends there: cipher and rot13 run forever. Every other example runs correctly.
- **[coronariac](https://github.com/pabloleonor/coronariac)**, in Java. It can't run a deck. Cell 00 starts out as
  000 rather than 001, so there's no loader, and nothing a deck holds ends up where it belongs. With 001 put in cell
  00 by hand it gets further, but its accumulator isn't one register. CLA sets one operand and ADD adds a second
  without carrying the sum forward, so a store after a load writes 0. Only hello and days come out right.

We haven't checked [Kaleberg's CARDIAC](http://www.kaleberg.com/software/cardiac/), which is a closed-source
download, or Al Williams's [spreadsheet](http://www.drdobbs.com/embedded-systems/cpu-design-on-paper/240153480)
and [FPGA](http://www.drdobbs.com/embedded-systems/paper-to-fpga/240155922) versions.

## The command

    cargo cardiac new NAME [cargo new's options]
    cargo cardiac setup
    cargo cardiac run [build's options] [--bin NAME] [-- INPUT...]
    cargo cardiac view [build's options] [--bin NAME] [-- INPUT...]
    cargo cardiac debug [ekg's options] FILE [INPUT]...
    cargo cardiac asm [carc's options] FILE
    cargo cardiac build [--manifest-path PATH] [-p NAME] [--release | --profile NAME] [--bin NAME]...
    cargo cardiac build [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] [--auto-overlay] [--edition YEAR] FILE.rs...
    cargo cardiac build --emit deck,listing,map,asm ...
    cargo cardiac help [COMMAND | CODE]

`new` is `cargo new`, with `cardiac` added and the countdown as `src/main.rs`. `setup` fetches the driver and its
nightly. `run` builds as `build` does and runs the deck, input from after `--` or from standard input, with the
build's messages on standard error, so standard output is the program's. `view` builds as `run` does and opens
the program in Cardigan, stopped at the start with the input in its reader. `debug` is heartburn's debugger and
profiler, ekg, on a listing or a deck, and `asm` is its assembler, carc, which makes a deck of a listing. Each
takes the options the program of its own does, and `--help` says them. `help` says the usage, `help COMMAND` a
command's part of it (for `debug` and `asm`, ekg's and carc's own), and `help CODE` what a message's code means.

The settings are cargo's. A package is built with the `dev` profile, `--release` with `release`, and `--profile
NAME` with a profile of the workspace's `Cargo.toml`, resolved by cargo itself. Its `opt-level` is rustc's, and it
also says how hard the compiler's own passes work: none at 0, all of them from 1 up. A single file has no
`Cargo.toml`, so it takes rustc's own flags instead, with rustc's defaults, and is Rust's 2021 edition unless
`--edition` names another.

`--emit` chooses what build writes beside each program: `deck` (`NAME.cqb`), `listing` (`NAME.cq`, the program in
Brian Stuart's six columns, which carc assembles), `map` (`NAME.map`) and `asm` (the compiler's own assembly,
`NAME.asm`). A listing and the assembly name each cell after the variable it holds, where the source has one (`n`,
or `main_n` when another function has an `n` too), and say each line as the Rust it came from (`height = cell / 10`).
The default is `deck,map`. `-q` says only errors, with no notes or warnings (rustc's included), and `-v` also says
where the cells go, by function, even when the program is too big to build. `--message-format=json` says it all as
JSON Lines on standard error, as cargo does.

Each message about a program has a code, as rustc's errors do: `error[RCE0013]` for an error, `warning[RCW0001]`
for a warning and `note[RCI0001]` for a note. The message is one line; `cargo cardiac help CODE` says what it
means on CARDIAC and what to write instead, as `rustc --explain` does, and so does the code's page in
[the message codes](https://lab.strangequery.com/rust-cardiac/messages/), whose address it ends with. After a build
with errors or warnings, the last line says where their codes are explained, as rustc's does. In JSON the code is
the message's `code`.

The target is `cardiac-unknown-none`, a notional triple: rustc doesn't know it, so there is no `--target` to give,
and `cargo build` builds natively. `cargo cardiac` is what builds for CARDIAC, with the CARDIAC driver as rustc.

## How it works

    Rust --rustc (the CARDIAC driver)--> MIR --cargo-cardiac, in rustc--> CARDIAC assembly --assembler--> deck

`cargo cardiac build` reads the package with `cargo metadata`, then runs `cargo check` on it with the profile, in a
target directory of its own, with the CARDIAC driver as cargo's rustc. The driver is rustc with a CARDIAC backend:
when rustc has analysed a program, the driver turns its MIR into the compiler's IR and compiles it in rustc's
process. The crates a program uses are built with `-Zalways-encode-mir`, so whatever `main` reaches is compiled,
whichever crate it is in.

The compiler (`src/compile`) works in these steps:

- **IR.** Each crate's MIR, from rustc_public inside rustc, is made into a typed IR (`ir`), named as rustc names it
  (`driver/src/convert.rs`). The crates' items are joined into one program (`crates`).
- **Front end.** The program is put in the terms the passes expect (`front`).
- **Passes.** Typed transformations on each function's blocks. `for` loops become counter loops and structs
  become plain locals; then range analysis drops the checks it can prove, copies are forwarded, repeated work is
  done once, sums are folded, INP and OUT go straight to array elements, and cells are shared between values never
  live together.
- **Lowering.** Each function becomes machine IR (`lower`, `machine`): typed lines of instructions, labels and data
  cells. The runtime routines the program needs are added (`runtime`).
- **Peephole.** The peephole rules and the block layout rewrite the machine lines (`peephole`).
- **Phases and deck.** A program in phases is split into its data and each phase's code (`phases`). The assembler
  places them from cell 03, and the deck and the panic map are written (`tape`, `panics`).

Calls use CARDIAC's own convention: JMP leaves a return instruction in cell 99, the callee keeps it, and `return`
jumps back through it. Indexing an array by a variable is self-modifying code, as it was by hand. Why things are the
way they are: `docs/DECISIONS.md`.

## Optimizer

On from opt-level 1 up. At opt-level 0, the default of cargo's `dev` profile and of rustc, it is off and the MIR
is lowered as rustc wrote it.

| test program              | literal lowering  | `cargo cardiac build` |
|---------------------------|------------------:|----------------------:|
| countdown                 |                54 |                     8 |
| Fibonacci                 |                70 |                    17 |
| add() in a loop           |                76 |                    33 |
| nested calls              | 104 (doesn't fit) |                    49 |
| FizzBuzz 1–100            | 285 (doesn't fit) |                    47 |
| arrays + borrowed array   | 225 (doesn't fit) |                    54 |
| &mut references           | 119 (doesn't fit) |                    77 |
| raw pointers              | 109 (doesn't fit) |                    36 |

Cells, for the programs in `tests/programs`. "Literal lowering" is MIR at opt-level 0, overflow checks on, with the
optimizer off; `cargo cardiac build` is opt-level 2, overflow checks off.

## If you think you found a bug

Email [bug@strangequery.com](mailto:bug@strangequery.com). Send the program, the input you gave it, what it printed
and what you expected, and the version from `cargo cardiac --version`. The best "expected" is what the same program
prints with `cargo run`.

## Developing rust-cardiac

[CONTRIBUTING.md](https://github.com/StrangeQuery/rust-cardiac/blob/HEAD/CONTRIBUTING.md) says how to build and test the compiler, and how to send us a pull request.

The compiler's test programs in `tests/programs` are single files that define the built-ins they call themselves,
as stubs the compiler knows by name, so they need no package and no `cardiac` crate:

```rust
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

pub fn main() { out(7); }
```

The tests run decks on burn, heartburn's runtime: the one on the PATH, or the one `BURN` names.

    ./test_compiler.sh       # every test program in tests/programs natively and on CARDIAC, outputs compared;
                             # and the programs that must not compile
    ./test_examples.sh       # every example: cargo run against its deck on burn
    ./test_features.sh       # one program per piece of Rust, in four settings: remakes tests/features/SUPPORT.md
    ./fuzz.sh                # the programs in tests/fuzz/targets on generated inputs, natively and on CARDIAC,
                             # every opt-level with overflow checks on and off (--runs N, --seed S, --only NAME)
    ./fuzz.sh --generate 200 # 200 generated programs, each built in those settings and run so
    ./fuzz.sh --repro F.rs -- 3 4   # one program built in every setting and run on that input, both ways
    ./ir_check.sh            # the driver's IR, built from rustc_public, against the IR parsed from rustc's MIR text
    cargo test               # the compiler's own unit tests

`cargo cardiac test` is the compiler's own test command, which the scripts use. It compiles a program as the tests
do, writes its IR printed as MIR (`.mir`) and its assembly (`.asm`) beside it, runs it from `main_bb0` on the
input, and prints the values it outputs on one line:

    cargo cardiac test [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] FILE.rs [-- INPUT...]
    cargo cardiac test [--manifest-path PATH] [-p NAME] [--release | --profile NAME] --bin NAME [-- INPUT...]

## Limits

Memory is 96 cells for code and data together, per phase. The compiler is pinned to rustc 1.97's MIR: the driver is
built on that version's last nightly, with `rustc-dev`, whose interfaces change from one nightly to the next, so a
newer Rust means porting the driver.
