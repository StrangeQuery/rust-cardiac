## 2026-11-05 · Literal panic messages go in the panic map; `assert_eq!` says Rust's first line

- **Decision.** A panic whose message is a literal (`panic!("negative card")`, `assert!(a > 0, "...")`,
  `unreachable!("...")`, `unimplemented!("...")`, `todo!("...")`) and every `assert_eq!` and `assert_ne!`, with a
  literal message or none, become in the front end the call a bare `assert!` makes, `core::panicking::panic(const
  "message")` (`panic_messages`), and `panic`'s string is the message the map gives the check, as `expect()`'s is.
  rustc builds a `core::fmt::Arguments` for the message: at opt-level 1 with a call to `Arguments::from_str` in
  the block before the panic, which becomes a goto; at opt-level 2 in the panic's block, its template the
  string's address and its `args` the length shifted with the low bit set, which is what marks a template that is
  only a string. The panic's block keeps no statements (nothing it did can be seen once the program has
  panicked), and the locals that only fed the message go, with their declarations: `assert_eq!`'s borrows of its
  two values, the `Arguments`, the string. `assert_eq!(a, b)` calls `core::panicking::assert_failed` with the two
  borrows; its message is Rust's first line, ``assertion `left == right` failed`` (`!=` for `assert_ne!`), and
  `: message` after it for one with a message. A bare `assert!(c)` and `unreachable!()` now say what Rust says
  (`assertion failed: a > 0`, `internal error: entered unreachable code`), where the map said `explicit panic`.
- **Why.** Seven rows of the support matrix were refused only for the message, and the comparison before the
  panic was already compiled (`assert!(a == b)` worked).
- **Cost.** Rust prints the two values under `assert_eq!`'s first line (`  left: 7` and ` right: 3`); the map
  doesn't. It is written before the program runs, and its format has one message a check, so it would need the
  cells the values were in at the panic, which the compiler shares with other locals by then. A message with
  `{}` arguments (`panic!("bad card: {}", a)`) is no literal and stays refused, as do `assert_matches!` and a
  message built any other way. No program grew: `assert_eq!(a, b)` is the same 20 cells as `assert!(a == b)`.
