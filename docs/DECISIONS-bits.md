## 2026-11-06 · The bit methods are the intrinsics rustc leaves, written as walks over the bits

- **Decision.** `count_ones`, `leading_zeros`, `trailing_zeros`, which rustc leaves as calls of the intrinsics
  `ctpop`, `ctlz` (`ctlz_nonzero`) and `cttz` on a u8, a u16 (an i16's) or a usize, and `rotate_left` and
  `rotate_right` of a u8 by a constant, are code of the caller's own (`bits`): the low ten bits of the cell walked
  from the top as `runtime::bitwise` walks them, until all that is left is 0. A cell below 0 stands for a value
  whose high bits are 1, a u16's (`wide`) or a usize's past a card: its ones are the bits less those of -1 - c, its
  leading zeros none, its trailing zeros those of -c. The u32 they return is a cell, as `wide` makes one of a count
  known to be on a card. A u8's rotation by one is 2v, less 255 past 255, done k times. `T::MAX >> n` by a variable,
  which `next_power_of_two` makes, is 0 doubled and 1 added (bits - n) times, checked for a usize, whose
  `usize::MAX >> 0` is usize::MAX. `is_power_of_two` is rustc's own code, `count_ones() == 1`. `wide` lets a shift by
  a u32 and a bit intrinsic read a value held modulo its size, which has the bits they read.
- **Why.** The counts are small and on a card; only the bits behind them aren't decimal, and the walk needs no
  more than adds.
- **Cost.** About 20 to 40 cells a call (the walk is written where it is called), and up to ten passes. Left refused: a
  usize's `leading_zeros` (rustc casts it to u64), rotations of an i16, by a variable, and `swap_bytes`, which on an
  i16 is past a card for most values (256 becomes 1, but 100 becomes 25600).
- **Evidence.** Every value on a card for the i16 methods, every u8 for the u8 ones (rotations by 0, 1, 3, 5, 7
  and 9), every value on a card cast to a usize, those below 0 too, against native Rust at all four settings. The
  support matrix's rows for them match at every setting but `swap_bytes_i16`.

## 2026-11-06 · Without overflow checks, the checks of core's inlined code are not made

- **Decision.** In a build without overflow checks, the driver makes an `assert` of an overflow of `+`, `-`, `*`, a
  negation or a shift go straight on to its target (`front::drop_inherited_overflow_checks`), and the checked
  operation it read the flag of, where nothing else reads it, is the plain one. rustc writes no such check in such a
  build, but keeps those of core's code it inlines that inherits the caller's overflow checks
  (`#[rustc_inherit_overflow_checks]`), and codegen leaves them out (`is_optional_overflow_check`). A divide's
  checks stay.
- **Why.** `u8::next_power_of_two(200)` is 0 natively without overflow checks, and panicked on CARDIAC at its
  `+ 1`.
- **Cost.** None found; an i16 past a card keeps three digits there, as `a + b` does in the program's own code
  without overflow checks.

## 2026-11-06 · `&`, `|` and `^` between integers, and shifts by a variable, are routines

- **Decision.** `a & b`, `a | b` and `a ^ b` of two integers (i16, i8, u8, a usize on a card), but for `x & m` by a
  mask one below a power of two, call a routine of the runtime's, one for each operator (`runtime::bitwise`). It
  reads a card as Rust's i16 is, 11 bits of two's complement, from the sign down: each operand is held as the bits
  not yet read, the low ten less 512, so that the next bit is whether that is 0 or more, and reading it is a
  doubling. The result is built as the bits come, 0 or -1 for the signs, then 2r and the bit, ten times. Two u8s
  have a routine of their own: eight bits, no sign. `a >> n` by a variable (i16, i8, u8, usize on a card) and `a << n`
  (i16, u8) call `rt_shr`, `rt_shl` and `rt_shl8`, with n taken modulo the type's bits first, as Rust's MIR does once
  the overflow check has passed: n halvings, rounding down; for an i16's `<<`, a's low 16 - n bits as a number of
  that many bits, doubled n times (513 << 7 is 128, as in Rust); for a u8's, n doublings modulo 256. rustc's check
  of a shift by an i16, `b as u16 < 16`, compiles: a u16 held modulo 2^16 compares with a constant on a card as the
  cell cast to a usize does (`wide::unsigned_compare`).
- **Why.** A cell holds a decimal number, not bits, so each operator is a loop over the bits; a loop of halvings
  would need the divide routine, and the doubling walk needs no more than adds.
- **Cost.** The routines are big: 66 instructions for `&` (67 for `^`), 49 for two u8s, 26 for `>>`, 46 for an
  i16's `<<`, and a few cells of data each; a program using one has about 25 cells of its own left. Slow: 10 passes
  of about 20 instructions for `&`. Where Rust's answer is past a card (`-513 & -512` is -1024, `512 | 511` is 1023,
  `300 << 2` is 1200), the program panics, overflow checks on or off, as `!` and `<<` by a constant do. Left
  refused: `<<` of an i8 or a usize.
- **Evidence.** Every pair of values on a card, for each operator, against Rust's answer (3.8 million each, at
  opt-level 2 without overflow checks), and every pair of u8s, at all four settings; shifts: every a on a card by n
  from -40 to 39 and more, at all four settings. The support matrix's `and_var`, `or_var` and `xor_var` match at
  every setting (95, 95 and 96 cells); `shift_var`, both shifts in one program, is too big.
