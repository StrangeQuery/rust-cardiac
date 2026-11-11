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

## Quick start
```
cargo install cargo-cardiac
cargo cardiac setup
cargo cardiac new countdown
cd countdown
cargo cardiac run
```

## Documentation

Everything else is on [lab.strangequery.com](https://lab.strangequery.com/rust-cardiac/):

- [rust-cardiac](https://lab.strangequery.com/rust-cardiac/): installing, debugging in Cardigan, the built-ins, and what CARDIAC is good and bad at.
- [Options and switches](https://lab.strangequery.com/rust-cardiac/options/): every `cargo cardiac` command, option and build setting, and the files a build writes.
- [The Language Reference](https://lab.strangequery.com/rust-cardiac/manual/): which language features just work, which run but should generally be avoided, and which are incompatible with CARDIAC.
- [Using rust-cardiac with vscode](https://lab.strangequery.com/rust-cardiac/vscode/): running and debugging the same program natively and on CARDIAC.
- [The message codes](https://lab.strangequery.com/rust-cardiac/messages/): what each error, warning and note means.
- [Demos](https://lab.strangequery.com/rust-cardiac/demo/): a voxel landscape and a ray-traced mirror ball, written in Rust and run on CARDIAC.
- [The `cardiac` crate on docs.rs](https://docs.rs/cardiac): `inp()`, `out()` and the other built-ins.
- [Cardigan](https://lab.strangequery.com/cardigan/), the CARDIAC IDE in your browser, and [heartburn](https://lab.strangequery.com/heartburn/), the runtime underneath.
- [The Strange Query CARDIAC Archive](https://lab.strangequery.com/cardiac/): CARDIAC itself, its history, programming tips and tricks.

The [examples](https://github.com/StrangeQuery/rust-cardiac/tree/HEAD/examples) folder has programs to read and run.

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

## If you think you found a bug

Email [bug@strangequery.com](mailto:bug@strangequery.com). Send the program, the input you gave it, what it printed
and what you expected, and the version from `cargo cardiac --version`. The best "expected" is what the same program
prints with `cargo run`.

## Developing rust-cardiac

[CONTRIBUTING.md](https://github.com/StrangeQuery/rust-cardiac/blob/HEAD/CONTRIBUTING.md) says how the compiler works,
how to build and test it, and how to send us a pull request.
