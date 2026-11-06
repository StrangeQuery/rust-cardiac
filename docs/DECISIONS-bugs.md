## 2026-11-05 · No card holds i16::MIN or i16::MAX: a test against one is settled, a value refused

- **Decision.** Before any pass, at every opt-level (`settled_by_the_card`), a comparison of an i16 with
  `i16::MIN` or `i16::MAX` is the constant the card makes it (`x == i16::MIN` false, `x < i16::MAX` true), with an
  `&` or `|` of it that one side settles and an assert of the result; and a switch on an i16 loses its arms past a
  card (`match x { i16::MIN => .. }`). Anything else that reads a limit is the value, which is refused as any
  constant past a card is: "constant out of range: i16::MAX (32767) is past what a card holds". The assembler
  refuses a `DAT` past a card too, should one get that far, rather than writing a deck burn can't load.
- **Why.** A limit used as a value (`i16::MAX - a`, or what `saturating_mul` saturates to) became a cell of 32767,
  and the compiler crashed running it (heartburn's "a card holds -999..999"); so did a test against one where
  lowering's own special case (`==` and `!=` with the limit on the right) didn't reach it. rustc writes these tests
  in its own checks (`-x`, `x / y`) and in the methods it inlines at opt-level 2.
- **Cost.** None measured: no test program or example changed size. `checked_neg`, `checked_div` and
  `checked_rem` on an i16 now compile at opt-level 2 (the test for i16::MIN goes, the test for 0 stays), and
  `saturating_mul` is refused where it crashed.
