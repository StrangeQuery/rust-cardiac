## 2026-11-05 · rustc's MIR inliner runs at opt-level 1 and s too

- **Decision.** For a crate built for CARDIAC at opt-level 1, s or z, the driver turns on rustc's MIR inliner
  (`-Zinline-mir`, unless the build sets that flag itself), so the MIR is what opt-level 2 makes: rustc runs the
  same MIR passes at 1 as at 2 and differs only in that one (`Inline::is_enabled`). Two kinds of call into core stay
  calls, marked `#[inline(never)]` in the session's view of core (`driver/src/inline.rs`, an override of
  `codegen_fn_attrs` for other crates' functions): the integer methods the compiler writes itself
  (`methods::written_here`), which keep the code they had at 1, `signum` and a u8's `abs_diff` among them, whose
  inlined forms (a `Cmp`, a subtraction in `i32`) are refused; and six methods whose inlined body the compiler
  compiles wrongly or crashes on today (`WRONG_INLINED`: `i16::midpoint`, `u8::overflowing_add`, and `i16`'s
  `checked_div`, `checked_rem`, `checked_neg` and `saturating_mul`), which stay refused at 1 rather than take the
  fault on. Once those are fixed at opt-level 2 they come off the list.
- **Why.** At 1 and s rustc left small library functions as calls (`Option::and_then`, `u8::checked_add`,
  `From::from`, `mem::take`, `bool::then`), and closures, generic functions and trait default methods too, and the
  compiler has no MIR to compile a call to core with: they were refused, the largest group of refusals in the
  support matrix, while at 2 the inlined body worked. One switch covers all of them, with rustc's own inlining
  rules, where writing each method here (as `methods` and `enum_methods` do) would be forty special cases.
- **Considered.** Converting each callee's MIR (core's, monomorphized) and compiling it as a function of the
  program: the calls would still pass `&mut self`, closures and whole structs, which the call path refuses. Keeping
  the program's own non-generic functions as calls (only core's, closures and generic functions inlined): it keeps
  opt-level 1 meaning "your helpers stay calls", but leaves trait methods taking a struct refused, and
  `#[inline(never)]` already keeps a call at every level. Raising the inliner's thresholds so that `map_or` and
  `filter` inline too: it inlines everything else more as well, and the examples grow (hello 8 to 23 cells) or
  stop compiling.
- **Cost.** Opt-level 1 and s are opt-level 2 in the MIR, so a helper is inlined there as at 2 (a helper called in
  several places is copied into each; `#[inline(never)]` keeps one copy). The test suite's run at opt-level 1
  covers calls less, and much of `enum_methods` (`is_some`, `unwrap_or`, `Try::branch` as calls) now runs only
  where rustc doesn't inline. docs/RUST.md's notes on what works "at opt-level 1 and s" need updating to say the
  levels now behave as 2 does.
- **Evidence.** Support matrix: 59 rows better, 0 worse; every row now gets the same outcome at opt-level 1 as at
  2 but for the eight above (signum and abs_diff work at 1 only, as before; the six stay refused at 1). Of the rows
  that matched before at opt-level 1, 103 settings shrank and 16 grew (`inline_always` and `inline_plain` 56 to
  85, as at 2; `enum_derive_partialeq` 71 to 77; `option_eq` 57 to 60; `checked_add` 38 to 39). The examples
  build at opt-level 2 and 3, so no listing of theirs changed. test_compiler.sh all MATCH, test_examples.sh all
  SAME, the fuzzer's targets in every setting no failure. `?` converting its error with `From` at opt-level 1
  now matches native Rust, as `From::from` is inlined.
