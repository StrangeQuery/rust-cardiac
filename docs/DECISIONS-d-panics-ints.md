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
