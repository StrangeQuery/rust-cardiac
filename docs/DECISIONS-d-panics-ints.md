## 2026-11-06 · A formatted panic message keeps its literal text, without the values

- **Decision.** `panic!("negative card: {}", a)`, and `assert!`, `unreachable!` and `assert_eq!` with a message
  that has arguments, become the call a bare `assert!` makes, as a literal message does, and the map gives the
  message's literal text with the values left out (`negative card: `). At opt-level 1 and 2 rustc builds the
  message in the panic's block: its template a byte string, which `panic_messages::template_text` reads in core's
  encoding (pieces, each its length and its bytes; placeholders, each a byte with the top two bits set and the
  fields its low bits say; a 0 byte at the end), and its arguments an array of `fmt::rt::Argument`s. It is
  refused unless the template reads to its end exactly, every placeholder names an argument there, no width or
  precision comes from an argument, every argument points to an integer, a bool, a char or a str (or a reference
  to one), and some text is left that isn't space (`panic!("{}", a)` has none).
- **Why.** Three rows of the support matrix were refused only for the message. The values can't be in the map,
  which is written before the program runs (as for `assert_eq!`'s), but the text says which panic it was.
- **Considered.** A value of the program's own type is formatted by its own `Display` or `Debug`, which could
  do what the program can see (call `out()`), and is dropped with the message: refused. A message built by calls
  (`Arguments::new` and `Argument::new_display` left as calls, at opt-level 0) is refused as before.
- **Cost.** None: the panic's block keeps no statements, as for a literal message. The three rows match at every
  setting; no test program or example changed.

## 2026-11-06 · A u16 that is compared is a cell that holds its value, never below 0

- **Decision.** `wide::u16_cells`, before `wide_locals`: the program's own u16 locals (not a parameter or the
  result, and not a local of a function rustc inlined) are i16 cells that hold the u16's value, where every line
  that names one is a cast to or from it, a copy, `+`, `-`, `*`, `/`, `%` (overflow-checked or not), a comparison,
  `&`, `<<` or `>>` by a constant, a switch whose arms are on a card, or an assert's message. The value is never
  below 0: a cast from an i8, an i16, an i32 or an isize is checked to be 0 or more and a cast from a usize to be
  at most 999 (a usize past a card is held below 0), and a difference is checked not to go below 0, in every
  profile. With overflow checks on that check is Rust's own (`attempt to subtract with overflow`); without them
  Rust's value is at least 65536 - 999, and it panics as a value past a card does (`the value doesn't fit on a
  card`). A group of locals that are named together is made cells whole or not at all, and is left to
  `wide_locals` where that makes every one of them a cell.
- **Why.** `wide_locals` keeps a u16 cast from an i16 as a number equal to it modulo 2^16, which is exact for what
  core's `abs_diff` does with one, but can't be compared: -3 held for 65533 is below 5. A u16 that is compared
  has to hold its value, and a u16's value is never below 0.
- **Cost.** Where Rust's u16 is past a card, CARDIAC panics or (a sum or product without overflow checks) keeps
  three digits, as for an i16: `(a - b) as i16` of 300 and 400 is -100 in Rust without overflow checks, and a
  panic here, where the modular cells would have been exact had nothing compared them. The matrix's int_u16
  matches at every setting; nothing else changed.
