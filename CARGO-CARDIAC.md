# cargo-cardiac

cargo-cardiac provides the `cargo cardiac` subcommands for [rust-cardiac](https://lab.strangequery.com/rust-cardiac/),
which compiles Rust programs for CARDIAC.

    cargo install cargo-cardiac

- `cargo cardiac new NAME` makes a package as `cargo new` does, with the `cardiac` crate as a dependency.
- `cargo cardiac setup` downloads the CARDIAC driver and the nightly it runs on, before the first build needs them.
- `cargo cardiac build` compiles the package's programs to CARDIAC decks, each with its panic map.
- `cargo cardiac run` builds a program and runs its deck, with input after `--` or from standard input.
- `cargo cardiac debug` steps through a listing or a deck, or profiles it.
- `cargo cardiac asm` assembles a listing into a deck.
- `cargo cardiac test` compiles and runs a program as the compiler's own tests do.

`cargo cardiac help` lists every option. For everything else, see the
[rust-cardiac page](https://lab.strangequery.com/rust-cardiac/) and the
[repository](https://github.com/StrangeQuery/rust-cardiac).
