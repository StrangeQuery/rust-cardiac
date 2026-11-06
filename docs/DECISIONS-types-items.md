## 2026-11-05 · A unit struct is no cells; the driver says which structs have no fields

- **Decision.** The driver writes down, beside the enums, each struct the crate's locals have, by its name and
  whether it has no fields (`facts::structs`, `structs::StructDef`). The struct pass gives a unit struct
  (`struct Flag;`) a layout of no fields: a local of it is no cells, a parameter no argument, a `&Flag` (a struct
  behind a shared reference, the struct by value) nothing, and its value (`Flag`, which MIR prints as a unit
  variant, or `const Flag`) nothing to write. A name a struct with fields also has, or a const, gets no unit layout,
  as the pass knows structs by their names.
- **Why.** The pass learns a struct's fields from the values that build it, and at opt-level 1 rustc leaves a unit
  struct a local that nothing builds (`debug d => const Doubler`): it was refused as a local of an unknown type.
- **Cost.** None: a unit struct took no cells where it compiled before (opt-level 2, rustc having dropped it).

## 2026-11-05 · A function that returns `!` is called with no way back

- **Decision.** A call of the program's own function declared `-> !` (`fn fail() -> !`), which MIR writes with no
  return block, is its arguments and the jump to the function; the function keeps no return instruction (no
  `CLA 99`, no exit cell), and a local of type `!` needs no cell.
- **Why.** It was refused as a terminator the backend doesn't have, though it is an ordinary call.

## 2026-11-05 · A function that can't unwind is compiled as any other

- **Decision.** A call or a check whose unwind is `terminate(abi)` (in an `extern "C"` function, where Rust aborts
  rather than unwinds a panic) is read as `unwind continue`: CARDIAC never unwinds, and a panic stops the machine.
- **Cost.** Natively such a panic aborts, exit code 134, where burn exits 101 as for any panic; the output is the
  same. The panic map can't say otherwise without a message burn reads as an abort.

## 2026-11-05 · A char is a usize that fits on a card

- **Decision.** The IR reads a char constant (`'b'`, `Const::Char`; the lexer a char literal as rustc writes one),
  and the front end makes every char a usize: its type, and each constant its code point. A char is compared,
  matched and cast (`as u8` mod 256, `as i16`, `u8 as char`) as a usize is. A literal past a card (`'Ϩ'`, 1000)
  is refused as any constant past a card is, as Jason decided on cargo-cardiac-support#27.
- **Why.** Rust has no arithmetic on chars, and a program can make one only from a literal or a `u8`: every char at
  run time is below 1000, never negative, where a usize lives. What it takes from a usize is what a usize below
  1000 does.
- **Cost.** A listing says `98_usize` where the source says `'b'`. `char::from_u32`, `from_digit`, `to_digit`,
  `is_ascii_digit` at opt-level 1 and the like stay refused (calls, or `u32`).

## 2026-11-05 · `i8`: a cell from -128 to 127

- **Decision.** An `i8` is one cell, as an `i16` is, its range -128..=127: a local, a parameter, an array's element,
  and an overflow-checked op's pair, checked against that range. With overflow checks off its `+` and `-` (and `-x`)
  wrap, as a `u8`'s do: the plain op, then 256 added or taken away until the value is back in range (`wrap_i8`),
  unless the range pass shows it already is. `as i8` keeps the value mod 256 from any integer type, `i8 as u8` adds
  256 to a negative one, `i8 as usize` is a usize past a card for a negative one, as an `i16`'s is.
- **Not supported.** An `i8`'s wrapping `*` (checks off), which can pass what the accumulator holds; an `i8` in a
  struct or an enum; `i8`'s methods.
- **Cost.** About 12 cells for each `as i8` and each wrapping op. Dividing two `i8`s checks `-128 / -1`, which an
  `i16`'s needn't (no card holds `i16::MIN`).
