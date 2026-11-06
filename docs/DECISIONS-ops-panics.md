## 2026-11-05 · `!`, `<<` and `>>` by a constant on an i16 and a u8, and `^` of two bools, as arithmetic

- **Decision.** `methods::inline_operators` writes each as the arithmetic it is in two's complement, before the
  passes, so the range pass and the rest see ops they know. On a u8: `!a` is `255 - a`; `a << k` is the low
  8 - k bits of a times 2^k (`(a & (2^(8-k) - 1)) * 2^k`, never past 255, so nothing wraps); `a >> k` is the
  quotient by 2^k. On an i16: `a >> k` is `a.div_euclid(2^k)`, worked as the quotient of a for a not below 0 and
  `-1 - (-1 - a) / 2^k` below it, so only values not below 0 are divided (one short unsigned loop, not the signed
  divide and remainder routines: 46 cells for the row, where `div_euclid`'s own code took 81). `a ^ b` of two bools
  is `a != b`. k goes up to 9 on an i16 (512 is the last power of two a card holds) and 7 on a u8; past that, by a
  variable, and on a usize they stay refused, now with a message that says so.
- **Where Rust's value is past a card.** Rust's `!` and `<<` never panic over the value (overflow checks look at
  the shift amount only), but on an i16 they can make a value no card holds: `!999` is -1000, and `a << k` is
  a * 2^k wrapped in 16 bits (`300 << 7` is -27136; `513 << 7` wraps all the way round to 128, which a card does
  hold). A card can't keep 16 bits, so the result is checked against a card with overflow checks on and off
  alike, and the program panics there (`the value doesn't fit on a card`), rather than keep the low three digits
  as an unchecked `+` does: `!a` is `assert(a < 999)` and then `-1 - a`, and `a << k` is `MulWithOverflow(a,
  2^k)` and its assert. So `a << k` panics wherever a * 2^k is past a card, even the few times the 16-bit wrap
  lands back on one: a panic, never a wrong answer. The range pass drops either check where it can prove the
  value fits (`!i` and `i << 3` of a loop counter 0..12 have none). `a >> k` is always on a card.
- **Measured.** Every value a card holds, through every k, on both types, at opt-level 1 and 2 with overflow
  checks on and off, compared with Rust's value: no wrong answer, and a panic only where Rust's value (for `<<`,
  a * 2^k) is past a card. Five matrix rows refused before match at every setting; no test program or example
  changed.

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
