## 2026-11-06 · A slice of run-time length is its address and a usize of its length

- **Decision.** `slices::run_time_lengths`, after `known_lengths`: each slice pointer (`&[T]`, `&mut [T]`, `*const
  [T]`) whose length isn't known where it is used is two locals, as Rust's `&[T]` is two words: the pointer itself,
  one cell holding the first element's address, and a usize beside it. A slice parameter is followed by its length,
  and every call of the function passes both (for an array pointer passed as a slice, the constant in its type);
  each definition of a slice local sets its length too, from the slice it is copied, cast or reborrowed from, or
  from `*const [T] from (p, n)`, which the IR reads as `SliceFromParts(p, n)` when n isn't a constant. `PtrMetadata(s)`
  is the length's local, so Rust's own bounds checks, which compare with it, stay where Rust has them; `(*s)[i]` is
  the self-modifying read an array by reference already is. The front end leaves `PtrMetadata` and
  `SliceFromParts` as the operations they are, not as tuple structs. `slice_index_fail`, the panic of a range out
  of a slice's bounds (`&t[..n]` with n past the end), is a panic, its message the bound that failed.
- **Why.** A slice passed to a function, a `&mut [T]` written by one and a subslice of a run-time length were
  refused, though all of it is code the compiler writes for `&[T; N]` once the length is a value.
- **Cost.** A slice parameter is a cell more than an array reference, and its length is passed at each call. A
  slice whose length can't be followed (a call's result, a field of a struct) keeps none: its `PtrMetadata` stays,
  and is refused, as is a call that would pass such a slice on.
- **Evidence.** The support matrix's slice_param, slice_mut, slice_on_slice, slice_len and slice_is_empty match at
  every setting; 0 rows worse; the compiler's tests and the examples unchanged.

## 2026-11-06 · Core's slice methods that rustc calls are the loops they stand for

- **Decision.** `slice_calls::write_calls`, between `known_lengths` and `run_time_lengths`: where rustc calls
  them (at both opt-levels), `split_at` (its pair read field by field, each field a slice of its own),
  `copy_from_slice`, `rotate_left`, `binary_search` and the insertion sort `sort_unstable` runs for 2 to 20
  elements are written in the IR as plain loops over the slice, with core's checks and messages (`mid > len`,
  the lengths not matching, `mid <= self.len()`). `binary_search` is core's own search step for step, which never
  stops early, so it finds what core finds among equal elements and in a slice that isn't sorted; on a slice of
  known length its halvings are known and written out, and on any other the halving is a loop of subtractions.
  `slice_calls::chunks_and_windows`: a `Chunks` or `Windows` whose `next()` rustc calls is two locals, the rest of
  the slice and k, and each `next()` the two slices core's makes; the `Option<&[T]>` it returns is its
  discriminant and its slice. `insertion_sort_shift_left`'s function-item type argument and `NonZero`'s
  pattern-typed constant (`2_usize is 1..`) are read. After the struct pass, a pointer to a whole local array
  (or a slice of it from its start) used only as `(*p)[i]` is the array (`pointers_to_locals`), so these loops
  index it directly. `(*s)[-k of n]`, `last()`'s, is a constant index, or an index worked out from the length.
- **Why.** Each is a short loop the compiler writes well, and core's own bodies aren't here, or (rotate, sort)
  move memory as a machine with a stack does.
- **Cost.** `rotate_left(k)` turns the slice k times by one, k times n moves, where core's moves each element
  once: the fewest cells (the support row is 87 cells). `chunks_mut`, `split_at_mut`, `rotate_right`, the other
  sorts and searches stay refused.

## 2026-11-06 · `find`, `max` and `min` on an array's iterator; a reference to a reference is a cell

- **Decision.** `iterators::iterator_methods` writes three more of the calls rustc leaves on a slice iterator
  into one array: `find` with a closure that captures nothing (the closure given a reference to the item, as
  core gives it), and `reduce` with the closures core's `max` and `min` pass, as a loop keeping the index of the
  greatest (the later of equals, as `max_by` keeps) or the least (the earlier). A reference to a cell that holds a
  reference to a card's value (`&&i16`, what `find`'s closure takes) is one cell, its address, read through twice.
- **Why.** Each is the loop `for i in 0..N`; the result is `Option<&T>`, which the enum pass already holds.
- **Cost.** None found. `find` on a closure that captures, and these on a slice of run-time length, stay refused.

## 2026-11-06 · A function that returns a slice returns its length in a cell of its own

- **Decision.** `run_time_lengths`: a function whose result is a slice keeps the slice's length in a usize of its
  own beside its result, one of its `results` (as a struct's fields are), and each call of it reads the length
  from that cell on the way back (`@f.n`, as the caller reads a struct's). A function whose result's length
  can't be followed is refused.
- **Why.** `fn tail(s: &[i16], k: usize) -> &[i16] { &s[k..] }` was refused; a `&str` a function returns is one.
- **Cost.** A cell in the callee, and a copy at each call.

## 2026-11-06 · A `&str` is the slice of its bytes

- **Decision.** `front::strings_as_bytes`, before byte strings are built: a `&str` local, parameter or result is
  a `&[u8]`, and a string literal it is set to (or `as_bytes()` of one) is the byte string of its bytes, which the
  slice passes then lay out as any other. Only literals of ASCII and the plain escapes; any other stays refused.
- **Why.** `.len()` of a literal, its bytes indexed, and a `&str` chosen at run time are slice operations the
  compiler now has.
- **Cost.** A literal is built as an array where it is used, two instructions a byte: `==` of two `&str`s is
  still refused (core's `compare_bytes`), and would be too big as built.
