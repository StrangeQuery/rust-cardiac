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

## 2026-11-06 · A tuple of tuples, and an array of tuples, are structs as a tuple of scalars is

- **Decision.** `structs::tuples_as_structs` makes a struct of a tuple whose elements are scalars or, now, such
  tuples (`((i16, i16), bool)` is `tuple__tuple2__i16_i16_bool`, whose field `0` is `tuple__i16_i16`), and of the
  element of an array of them (`[(i16, i16); 3]` is `[tuple__i16_i16; 3]`, which the struct pass keeps as one
  array per leaf, as it does an array of structs). The tuple type is replaced wherever a local's type or a field's
  (`(_6.0: (i16, i16))`) has it; `(a, b)` into any place of such a type is the struct built, and a tuple constant
  there (`[const (0_i16, 0_i16); 3]`) the struct's constant. A function whose parameter is a tuple of tuples is
  read, as one of scalars was.
- **Why.** The struct pass already takes apart structs of structs and arrays of structs; a tuple is a struct whose
  fields are numbered.
- **Cost.** None to other programs: the compiler's tests, the examples and the matrix's other tuple rows are
  unchanged. An array of tuples passed by reference stays refused, as an array of structs is.
- **Evidence.** The support matrix's array_tuples and tuple_nested_return match at every setting. Probes matching
  native Rust at the four settings: an array of `(i16, bool)` set whole from a tuple local and by field, copied
  out whole; a literal array of tuples indexed; a `((i16, i16), i16)` parameter given a tuple built and a tuple
  local; `(usize, u8)` and `(bool, (u8, i16))` returned; an array of `((i16, i16), i16)` set whole and read by
  field; an array of `(u8, i16)` wrapping its u8s.

## 2026-11-06 · `char::to_digit` and `char::from_digit` are written here, their u32s cells

- **Decision.** `chars::char_digits`, in the front end after a char is a usize: a call of either with a constant
  radix from 2 to 36 is the tests core's makes, written in the function. `c.to_digit(R)` is `Some(c - '0')` for c
  in '0'..'0' + R (up to '9'), `Some(c - 'a' + 10)` and `Some(c - 'A' + 10)` for the letters below R, else `None`;
  its `Option<u32>` is an `Option<u8>` (the definition made from the program's `Option<u32>`), and each read of its
  digit a u8 cast to u32, which `wide` holds as a cell, its value 0..=255. That is done only where the result is
  read by its discriminant and its digit alone; anything else done with it stays refused. `char::from_digit(n, R)`
  is `Some` of '0' + n (or 'a' + n - 10) for n below R, else `None`: n is compared with R and cast to a u8, which
  `wide` does with a cell that holds n modulo 2^32, and below R the cell is n. An enum's field that is a char is a
  usize, as every char is by then.
- **Why.** The digit and its u32 are on a card; only the types weren't.
- **Cost.** to_digit is 4 instructions a range it tests (one for radix 10 or less, three above), and a u8's cast
  of the digit; the matrix's rows are 48 and 32 cells. A radix that isn't a constant stays refused.
- **Evidence.** char_to_digit and char_from_digit match at every setting. Every byte 0..=255 through `to_digit`
  with radix 2, 8, 10, 16 and 36, and every n from -40 to 40 (and 500, 999, -999) through `from_digit` with the
  same radixes, match native Rust at the four settings; so does a digit unwrapped and one compared with `Some(3)`.

## 2026-11-06 · A tuple in an enum is the struct the struct pass makes of it; `Option::zip` is written here

- **Decision.** The enum pass keeps a variant's field that is a tuple of scalars (or of such tuples) as the tuple,
  and the struct pass makes it its struct (`structs::tuple_struct`), as it makes the functions' tuples, so that
  `Option<(i16, i16)>` is a tag and the pair's two cells. The enum pass had named the struct itself, which the
  struct pass then never made. `a.zip(b)`, which rustc calls at every opt-level, is core's `match (a, b) {
  (Some(x), Some(y)) => Some((x, y)), _ => None }`: each discriminant tested, and `Some` of the pair built
  (`enum_methods::zips`). A function whose parameter is an `Option` or a `Result` of such a tuple is read.
- **Why.** Each piece was there: tuples as structs, structs in enums, and the tests of a discriminant.
- **Cost.** None to other programs: the compiler's tests, the examples and the matrix's option, result and enum
  rows are unchanged.
- **Evidence.** option_zip matches at every setting (44 and 69 cells). Probes matching native Rust at the four
  settings: a zip of an `Option<i16>` and an `Option<bool>` matched on both, `Option<(i16, i16)>` returned from a
  function and `Option<(i16, u8)>` passed to one, and an `Option<((i16, i16), bool)>`.

## 2026-11-06 · `<` between two `Option`s: a comparison's constant `Some(Equal)` is its number

- **Decision.** The C-like pass numbers a variant named alone inside another enum's constant,
  `const Option::<Ordering>::Some(Equal)` (rustc's `partial_cmp` of two `Option`s, inlined), by the one C-like
  enum among the constant's type arguments that has a variant of that name. And where the for-loop pass makes an
  `Option` that is only ever read as `Some`'s value its value (`Some(x), unwrapped at once`), a constant `Some(c)`
  it is set to elsewhere is `c` too; before, that line was left, and the program refused.
- **Why.** `o < p` of two `Option<i16>`s is a choice among four `Some` of an `Ordering`, read straight back.
- **Cost.** None to other programs.
- **Evidence.** option_lt matches at every setting (69 cells). `<`, `<=`, `>` and `>=` of two `Option<i16>`s over
  every pair of `None`, below, equal and above, an `Option<u8>` against `Some(3)` and `None < o`, match native
  Rust at the four settings. `o.cmp(&p)` and `o.max(p)` stay refused (core's `Ord` for `Option`, a call).
