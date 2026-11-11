# Building and testing rust-cardiac

This is how we build the compiler from source, how we test it, and how to send us a change. If you only want to
compile programs for CARDIAC, you don't need any of this. The quick start in the README is enough:

    cargo install cargo-cardiac
    cargo cardiac new countdown
    cd countdown
    cargo cardiac run

## What you need

- **rustup**, not a distribution's packaged Rust. The repository uses two toolchains: stable Rust 1.97 or later for
  `cargo-cardiac`, and the nightly that `driver/rust-toolchain.toml` pins for the driver. rustup installs that
  nightly the first time you build in `driver/`, with its `rustc-dev` component. That is about 1.5 GB.
- **burn**, heartburn's runtime, on your PATH (or named with `BURN`). The tests run every deck on it. It comes from
  `brew install heartburn` (from Strange Query's tap, `brew tap strangequery/tap`), from the release archives of
  [heartburn](https://github.com/StrangeQuery/heartburn/releases), or from a checkout of heartburn with
  `cargo build --release --features tools --bins`.
- **On Windows with the GNU toolchain**, MinGW's binutils (`as` and `dlltool`, from MSYS2's
  `mingw-w64-x86_64-binutils`) on your PATH, to build the driver. The MSVC toolchain doesn't need them.

## Building

There are two programs. `cargo-cardiac` is the command. The driver, in `driver/`, is rustc built as a library: it
does the compiling, inside rustc's own process. `driver/` is a workspace of its own because it builds on the
pinned nightly.

    git clone https://github.com/StrangeQuery/rust-cardiac
    cd rust-cardiac
    cargo build --release                       # target/release/cargo-cardiac
    (cd driver && cargo build --release)        # driver/target/release/cardiac-driver

A `cargo-cardiac` built in the repository finds the driver in `driver/target/release` by itself. To use them from
anywhere, install both: `cargo install --path .`, then `cd driver && cargo install --path .`.

## Testing

Every test asks the same question: does the program print the same thing on CARDIAC as it does natively? A test
runs each program with `cargo run` (or as a native build), builds its deck with `cargo cardiac`, runs the deck on
burn with the same input, and compares the two outputs. A panic counts too: it has to happen both ways, with exit
code 101 (134 for a broken unsafe precondition, as Rust aborts).

Run these before you send a change. All of them must pass.

    cargo test              # the unit tests
    sh test_compiler.sh     # every program in tests/programs, at opt-level 1 and 2: each line MATCH or DIFFER
    sh test_examples.sh     # the example programs, with their inputs: each line SAME or DIFFER
    sh test_features.sh     # the feature tests in tests/features: one program per piece of Rust, in four settings

`test_compiler.sh` also checks the programs the compiler must refuse, and that each one is refused with the right
message. It stops at the first DIFFER.

`test_features.sh` runs one small program for each piece of Rust, in the order of the Rust Reference, at opt-level
1 and 2 with overflow checks on and off. It rewrites `tests/features/SUPPORT.md`, the table of what compiles, from
what happened, and the results beside it. Your diff then shows which rows changed. To check a change without
touching the committed results, write them somewhere else and compare:

    SUPPORT_OUT=/tmp/after sh test_features.sh
    sh test_features.sh --compare tests/features /tmp/after    # each row that changed, better or WORSE

No row may get worse. `sh test_features.sh NAME` runs only the rows whose path contains NAME.

The listings in `tests/programs/*.asm` and the decks beside the examples are checked in, so a change to the
compiler shows up as a change to them in your diff. That diff isn't a failure. We judge a change by the output and
by the cells it uses. A listing that changes and still prints the same is fine. If yours uses fewer cells, say so
in the pull request.

There are deeper checks for changes that go further:

- **`sh fuzz.sh`** runs the programs in `tests/fuzz/targets` on generated inputs in every setting.
  `sh fuzz.sh --generate 300` writes 300 random programs of its own and runs those. Run it for any change to code
  generation or to the passes. A finding comes with a `--repro` command that builds and runs it again.
- **`sh ir_check.sh`** compares two routes to the compiler's input: the IR the driver builds from rustc's own data
  structures, and the IR parsed from rustc's MIR text. Run it if you touch `driver/src` or `src/compile/ir`.
- **`sh ice_check.sh MARK`** reports any rustc internal error newer than the file MARK. Touch a file before your
  test runs, then check against it. rustc can report an internal error after a test has already passed.

Two runs of `test_compiler.sh` at the same time share a directory under `$TMPDIR`. Give a second run its own
`TMPDIR`.

The compiler's test programs in `tests/programs` are single files that define the built-ins they call themselves,
as stubs the compiler knows by name, so they need no package and no `cardiac` crate:

```rust
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

pub fn main() { out(7); }
```

`cargo cardiac test` is the compiler's own test command, which the scripts use. It compiles a program as the tests
do, writes its IR printed as MIR (`.mir`) and its assembly (`.asm`) beside it, runs it from `main_bb0` on the
input, and prints the values it outputs on one line:

    cargo cardiac test [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] FILE.rs [-- INPUT...]
    cargo cardiac test [--manifest-path PATH] [-p NAME] [--release | --profile NAME] --bin NAME [-- INPUT...]

## How the compiler works

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

## Where things are

- `src/compile/` is the compiler: the IR, lowering, the passes, and the machine that lays out cells.
- `src/codes.rs` is every message the compiler gives about a program, each with its code and explanation. A new
  message goes there (its top says how), and the website's pages are made from it, from what `cargo cardiac help
  --json` prints (`explain --json`, its old name, still works).
- `driver/src/` turns rustc's MIR into the compiler's IR, and holds the oracle that `ir_check.sh` runs.
- `cardiac/` is the crate programs depend on: `inp()`, `out()` and the rest of the built-ins.
- `tests/features/` is the feature tests and their harness. `tests/features/SUPPORT.md` is the table they make,
  and `NEVER.md` beside it explains the rows CARDIAC can never support. The top of `tests/features/src/main.rs`
  says how a row is written.
- `docs/manual.md` is the user manual, with a chapter for each part of the language in `docs/manual/`, in the
  Rust Reference's order. Its "rust-cardiac at a glance" section goes area by area: what works well, what costs
  more than it looks, and what is refused.
- `docs/DECISIONS.md` is the decision log: what we chose, why, and what it cost, newest first.

## Sending a change

1. **Open an issue first**, unless the fix is small. Say what you saw. For a wrong answer, give the smallest program
   you can find, its input, and what it printed natively and on CARDIAC. A program that prints the wrong thing on
   CARDIAC is the most important kind of bug, and we fix those before we work on making programs smaller.
2. **Fork the repository and make a branch.** Format with `cargo fmt`. `rustfmt.toml` sets lines of 120 characters.
3. **Add a test.** For a bug, put a program in `tests/programs` that fails before your change and passes after it.
   For a new piece of Rust that now compiles, do the same, add it to its chapter of the manual and to its area in
   "rust-cardiac at a glance" in `docs/manual.md`, and add a row for it in `tests/features` if it doesn't have one.
4. **Run the tests above.** All of them must pass. Run `fuzz.sh` too if you changed code generation or a pass.
5. **Write the decision down.** If your change decides something, such as how a type is laid out or what a pass
   may assume, add an entry to the top of `docs/DECISIONS.md`. Copy the shape of the entries already there: the
   decision, why, what it costs, and what you measured.
6. **Open the pull request.** Say what changed and why, how you tested it, and what happened to program sizes (for
   example, "sort 71 to 68 cells, nothing grew").

Write comments and commit messages in plain, full sentences. Keep comments sparse, as in the code around them: say
what a block is for and what isn't obvious, not what each line does.

By sending a pull request, you agree that your contribution is licensed under the MIT licence, the same as the
rest of the repository.

## Talking to us

Questions, ideas and half-finished thoughts are welcome on [Discord](https://discord.gg/VsqKEdBYzJ). Email bug
reports to [bug@strangequery.com](mailto:bug@strangequery.com). Proposals go in
[the issues](https://github.com/StrangeQuery/rust-cardiac/issues).
