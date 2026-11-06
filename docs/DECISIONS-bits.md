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
