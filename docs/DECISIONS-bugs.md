## 2026-11-05 · An integer method of core's that works in i32 is refused

- **Decision.** The IR keeps, for each body, the locals rustc declared in a function it inlined and that
  function's path, as the scope says it (`scope 3 (inlined core::num::<impl i16>::midpoint) {`; `Body::inlined`,
  read from the text and from rustc's source scopes alike). A function into which rustc inlined an integer method of
  core's, of a type other than i32, that has an i32 local (`i16::midpoint`, which adds the two in i32) is refused:
  "core::num::<impl i16>::midpoint, inlined in main, works in i32 so that it can't overflow, and on CARDIAC an i32
  is a card".
- **Why.** i32 is i16 on CARDIAC (`front::I32IsI16`), which is right for a program's own i32: on a card either way,
  and past one it differs as an i16 does. But core widens to i32 only so as to work past what the narrower type
  holds, and on a card that is a wrong answer for values the program never sees: `999.midpoint(998)` printed 598.
  An i32 method's own i32 (a range of i32's step, `<impl i32>::overflowing_add_unsigned`) is the program's, and
  stays.
- **Cost.** `i16::midpoint` doesn't compile at any opt-level (at 1 and s it was already a refused call).

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
