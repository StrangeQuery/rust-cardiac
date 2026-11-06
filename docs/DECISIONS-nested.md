## 2026-11-06 · An array of arrays is one array, row after row

- **Decision.** `nested::arrays_of_arrays`, before any other pass: a `[[T; W]; H]` of scalars (and a reference or a
  pointer to one) is a `[T; H * W]`, as Rust lays it out, and `t[r][c]` is `t[r * W + c]`. The index is worked out
  on the line before the one that names the element, after rustc's own checks of r and c, so the bounds checks
  stay Rust's: a constant index where both are constants, `t[c + r * W]` (an index plus a constant) where r is, and
  else r * W and r * W + c in locals of their own, one for each such part, which a line takes as they are where
  every way into it has worked them out and neither index has changed since. A grid built of its rows (`[move _2;
  3]`, `[move _2, move _3]`, each row built in the block just before and read nowhere else) is its elements written
  out. A whole grid may be copied, borrowed, passed to one of the program's functions and returned from one.
- **Why.** One flat array indexed `r * W + c` is what a program for CARDIAC writes by hand, and the compiler has
  everything it needs for that; only the arithmetic was missing.
- **Refused.** Anything else done with a row: borrowed (`&t[r]`, `t.iter()`), copied out (`let row = t[1]`), or a
  row read again after the grid is built of it, as GVN can leave one; then every array of arrays in the program
  stays as it is, refused as before, since one grid's layout has to agree with every function it is passed to.
- **Cost.** Two to five instructions an index, about what `t[r * W + c]` written by hand costs (a program that
  fills a 2-by-3 grid in two `for` loops and reads one element is 68 cells; the hand-flattened program is no
  smaller). No program without arrays of arrays changes.
- **Evidence.** The support matrix's array_2d_nested (83 and 95 cells) and array_arrays match at every setting.
  Probes matching native Rust at the four settings, bounds failures too: a u8 grid wrapping, a bool grid, a grid
  passed by `&` and by `&mut` and indexed there, filled in nested `for` loops, copied whole and then changed, a
  literal grid, an index changed between two uses, and constant and variable indexes mixed in a loop.
