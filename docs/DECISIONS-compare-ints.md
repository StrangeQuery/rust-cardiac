## 2026-11-05 · `Ordering` is a C-like enum; `Cmp` is two tests

- **Decision.** rustc's three-way compare is an operation of the IR, `BinOp::Cmp` (the driver used to hand it on
  as a variant named `Cmp`). The front end makes each `d = Cmp(a, b)` two tests before the C-like pass, `a < b`
  then `a > b`, setting d to `Ordering::Less`, `Greater` or `Equal`, which the C-like pass then numbers from the
  driver's facts (-1, 1, 0); the block is split at the `Cmp`. At opt-level 1, `<T as Ord>::cmp(&a, &b)` on an
  integer is that `Cmp` of what the references borrow, `Ordering::is_lt` and its kin are a comparison with
  `Equal`, `then` is a test of the first for `Equal`, and `reverse` is `Cmp(Equal, o)`. A local holding a C-like
  enum's discriminant in a type the backend lacks (`Ordering` is `#[repr(i8)]`) is an i16, with the constants it
  is compared with, but only where it is set by `discriminant` alone and only compared, switched on or cast:
  those give the same answer in both types, where arithmetic in an i8 could overflow where an i16 doesn't. A
  variant rustc names alone (`const Less`, in core's code it inlines) is the one of the enum it is set to or
  compared with.
- **Why.** `a.cmp(&b)`, `match` on an `Ordering`, `is_lt`, `then`, `partial_cmp` and `signum` at opt-level 2,
  and the derived `Ord` and `PartialOrd`, all were refused for want of `Cmp`, and an `Ordering` is a cell already.
  Two tests, not a subtraction and its sign: a - b may be past a card (`999.cmp(&-999)`).
- **Cost.** Two tests and three stores of a constant: the matrix's `out(a.cmp(&b) as i16)`, three times round, is
  27 cells at both opt-levels, as is the `match` on it, where jump threading takes each constant straight to its
  arm. No test program or example changed. Still refused: `partial_cmp` at opt-level 1 (it builds an
  `Option<Ordering>` in a way the enum pass refuses), and the derived `Ord` and `PartialOrd` methods rustc calls
  at opt-level 1 for a C-like enum (`<Lvl as Ord>::max`, `<Lvl as PartialOrd>::lt`), whose bodies are not compiled.

## 2026-11-05 · `i16 as u8` wraps as Rust's does; a usize compared with a constant past a card

- **Decision.** `x as u8` of an i16 is x mod 256, as Rust's two's complement gives it: a negative x is raised by
  256s, then the value lowered by 256s until it is below 0 and raised once (the code `usize as u8` already had for
  a usize past a card). Where the range pass knows x is 0..=255 the cast is a copy. The cast now takes the type of
  what it casts from the place, so an array's element or what a reference points to is cast by its type: before,
  `t[i] as u8` of an i16 element was stored as it was (-1 stayed -1, #139). A usize compared with a constant k
  past a card (1000 up to 2^64 - 1000), as rustc's inlined `i16::try_from(n)` tests `n > 32767`, is compared with
  999: a usize on a card is at most 999, and one past a card, held below 0, is above every k.
- **Why.** `i16 as u8` was refused, and so `u8::try_from` of an i16 at opt-level 2 (the range check, then the
  cast), and every row that has to read its u8s from a table; `i16::try_from` of a usize at opt-level 2 was refused
  for its constant.
- **Cost.** A cast the range pass can't settle is about 12 instructions and a pass of a three-instruction loop
  per 256 (at most four). `TryFrom` at opt-level 1, where rustc calls `try_from`, is still refused: the call
  builds a `Result` whose `Err` holds a `TryFromIntError`, a struct the enum pass has no cells for.
