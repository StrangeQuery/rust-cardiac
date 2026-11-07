## 2026-11-06 · A multiply by a negative constant takes the product away from 0

- **Decision.** `x * -k`, for a constant k that isn't the routine's, is x taken away k times from 0 (with the decimal
  shifts in between, as for `x * k`), not `x * k` stored and then taken from 0. A value carried in the accumulator
  for it goes to its cell first.
- **Why.** Stored, a product past a card kept three digits before anything read it: with overflow checks on,
  `999 * -9` gave -991 and `99 * -19` gave -881 where they panic (the check saw what was left), and an `i8`'s
  `wrapping_mul(-39)` wrapped those three digits, not the product: `-128i8.wrapping_mul(-39)` was -32, not -128.
  Taken from 0, the whole product is in the accumulator, as it is for a positive constant.
- **Cost.** Two instructions fewer for each: the product's store and its reload go, and a `CLA 0` comes first.

## 2026-11-06 · A wrapping `*` past a card is what CARDIAC keeps of it

Supersedes "A wrapping `*` is refused where the product can pass the accumulator" (2026-11-06), on Jason's ruling:
the machine has a standard, and we don't get to choose.

- **Decision.** A `u8`'s or an `i8`'s wrapping `*` (overflow checks off, or `wrapping_mul` inlined) that the range
  pass hasn't shown stays in its type's range compiles, whatever its factors. By a constant from -39 to 39 it is
  as before: the product is whole in the accumulator (9999 at most) and wraps as Rust's does. Any other is the
  multiply routine's, the constant as the count of adds: where the product passes a card, what is left of it is
  CARDIAC's own rule for a value past a card, the rule an `i16` has without overflow checks: the cell keeps its
  last three digits, with its sign, as 999 + 1 stores 000. The type's ordinary wrap is then taken of those three
  digits, as of any value on a card, so the result is always in the type's range: `200u8.wrapping_mul(200)` is
  40000, stored 000, so 0 (Rust: 64); `255u8.wrapping_mul(255)` is 65025, stored 025, so 25 (Rust: 1); `-128i8 *
  100` is -12800, stored -800, so -32 (Rust: 0). A product from -999 to 999 wraps exactly as Rust's does. Both
  factors known (rustc folds these, but a pass of ours could leave one) is Rust's own wrapped product.
- **Why.** CARDIAC has no multiply: a product is a loop of adds, and past a card the machine's overflow rule
  decides it, as the manual and the reference simulators have it (ADD and SUB in a four-digit accumulator, STO keeping
  the last three digits with the sign: Brian Stuart's simulator stores ndig(val, 3), CardiacVisualizer
  accumulator % 1000, burn wraps the accumulator at 10000). The routine stores its partial product after every
  add, each a card's three digits plus a factor, so the accumulator never passes four digits and the three digits
  left don't depend on how a machine bounds its accumulator: burn and a simulator that doesn't bound it give the
  same. The routine, not adds and shifts in place, for a big constant: in place the accumulator would hold more of
  the product than a card, and what the program later did with it would depend on whether it was read from the
  accumulator or from its cell. Jason: "Somebody somewhere will exploit that behavior for fun and profit. Let
  'em."
- **Cost.** Nothing for what compiled before, and overflow-checked builds are unchanged: there a `u8` product past
  255 panics, as Rust's does. The routine is slow for a big count (up to 255 passes of its loop), which only code
  that was refused before pays.
