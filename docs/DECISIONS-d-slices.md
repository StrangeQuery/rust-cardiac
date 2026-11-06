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
