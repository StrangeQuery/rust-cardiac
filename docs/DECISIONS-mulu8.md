## 2026-11-06 · A multiply by a negative constant takes the product away from 0

- **Decision.** `x * -k`, for a constant k that isn't the routine's, is x taken away k times from 0 (with the decimal
  shifts in between, as for `x * k`), not `x * k` stored and then taken from 0. A value carried in the accumulator
  for it goes to its cell first.
- **Why.** Stored, a product past a card kept three digits before anything read it: with overflow checks on,
  `999 * -9` gave -991 and `99 * -19` gave -881 where they panic (the check saw what was left), and an `i8`'s
  `wrapping_mul(-39)` wrapped those three digits, not the product: `-128i8.wrapping_mul(-39)` was -32, not -128.
  Taken from 0, the whole product is in the accumulator, as it is for a positive constant.
- **Cost.** Two instructions fewer for each: the product's store and its reload go, and a `CLA 0` comes first.
