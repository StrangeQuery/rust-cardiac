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
