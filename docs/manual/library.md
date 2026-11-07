# The core library

[The manual](../manual.md) › The core library

The Rust Reference leaves the library to its own documentation. This page says which parts of `core` work on
CARDIAC. Where rustc calls a library method rather than inlining it, the compiler writes the code rustc would have
inlined.

## What's different on CARDIAC

- No card holds `i16::MAX` or `i16::MIN`, so `checked_add` is `None` past 999 and the saturating methods panic past a
  card.
- The bit methods walk the cell's bits one at a time, so they cost cells and time.
- `i16::midpoint`, `swap_bytes` and `i16::from_str_radix` are refused, and some iterator adaptors aren't done yet.

## Integer methods

**Works well:** `abs`, `wrapping_abs`, `unsigned_abs`, `abs_diff`, `signum`, `is_negative`, `is_positive`; `min`,
`max`, `clamp`, `cmp`; `checked_` `add`, `sub`, `mul`, `div`, `rem`, `neg` and `pow`; `wrapping_` `add`, `sub`,
`mul`, `div`, `rem` and `neg`; `saturating_` `add`, `sub` and `mul`; `overflowing_add` and `overflowing_sub`;
`div_euclid` and `rem_euclid`; `pow`, `isqrt`, `ilog10`; on a `u8` or a `usize`, `is_power_of_two` and
`next_power_of_two`; a `u8`'s `midpoint`; `from`, `into`, `try_from` and `try_into` between the integer types that
work; `BITS`, `u8::MIN` and `u8::MAX`.

No card holds `i16::MIN` or `i16::MAX`, so `abs` has nothing to check. An `i16`'s saturating methods panic past a
card, as its plain arithmetic does, and `checked_add` is `None` past 999.

**Works, but costs more than it looks:**

| Method | What it costs | Cheaper |
|--------|---------------|---------|
| `count_ones`, `leading_zeros`, `trailing_zeros`, a `u8`'s rotations | They walk the cell's bits: about 20 to 40 cells a call, written where it is called, and up to ten passes. The build warns about them (RCW0001). | A table, or arithmetic on the digits. |
| `.rem_euclid()` by a variable | The divide routine, as for `%`, and about 10 cells more to make a negative remainder positive. | By a constant: a power of ten is shifts, as for `%`. |

**Refused:**

- `i16::midpoint`, on purpose. rustc adds the two in `i32` and halves, and past a card that sum can't be held, so
  the answer would be wrong. A `u8`'s works. Use `(a + b) / 2` where the sum stays on a card.
- `swap_bytes`, because CARDIAC can't hold the answer: 100 becomes 25,600.
- `i16::from_str_radix`, because a program's input is numbers on cards.

**Not done yet:** rotations of an `i16`, or by a variable, and a `usize`'s `leading_zeros`. Use an `i16` or a `u8`.

## Option and Result

`is_some`, `unwrap`, `expect`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `map`, `map_or`, `and_then`,
`filter`, `ok_or`, `or`, `xor`, `zip`, `take`, `replace`, `insert`, `get_or_insert`, `as_ref`, `as_mut`, `copied`,
`map_err`, `ok`, `err` and the rest work, and so does `?`, also converting the error with `From`. `==` and `<` of
two `Option`s work. `==` of two `Option`s tests both variants and then the payloads. For how they are laid out, see
[Types](types.md#enums).

## Iterators

**Works well:** on a range or an array's iterator, `sum`, `count`, `fold`, `max`, `min`, `find`, `position`, `all`,
`any`, `last` and `nth`. Chains of `map`, `filter`, `enumerate`, `skip`, `take`, `rev`, `chain`, `zip` and
`step_by`, from a range, an array, `once`, `repeat` or `successors`, consumed by `sum`, `count`, `fold`,
`for_each`, `max` or `min`. A chain is compiled as the one loop it stands for.

**Not done yet:**

- `filter_map`, `flat_map`, `peekable`, `scan` and `take_while`;
- `any`, `all`, `find` and `position` on a chain of adaptors;
- a chain kept and stepped by hand, or in a `for` loop;
- `rev` of `enumerate`, `skip`, `take`, `zip`, `step_by` or `successors`.

Use a `for` loop over a range, or the method on a range or a plain array iterator.

## Arrays and slices

`len`, `is_empty`, `get`, `first`, `last`, `contains`, `fill`, `swap`, `reverse`, `rotate_left`, `sort_unstable`,
`binary_search`, `split_at`, `copy_from_slice`, `chunks` and `windows` work. So do `[T; N]::map`,
`core::array::from_fn`, and `==` of two arrays.

## Strings

A string literal's `len()`, its bytes (`as_bytes()`) and indexing work. Comparing strings is refused: two short
literals compared come to about 124 cells. So is parsing them. Use numbers in place of words.

## The rest of core

`core::mem::swap`, `replace` and `take`; `Ordering` and its methods, `then`, `partial_cmp` and
`core::cmp::Reverse`; `bool::then` and `then_some`; `char::to_digit`, `char::from_digit` and `is_ascii_digit`; and
`core::hint::black_box`, `assert_unchecked` and `unreachable_unchecked` work.

Feature tests: [integers](../../tests/features/integers) and [library](../../tests/features/library).
