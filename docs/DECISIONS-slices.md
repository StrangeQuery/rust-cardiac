## 2026-11-05 · A slice of a known array is the array; a slice iterator's pointers are indexes

- **Decision.** `slices::known_lengths`, before the for-loop pass: a slice (`&[T]`, `*const [T]`, `NonNull<[T]>`)
  every definition of which comes, through copies, casts and reborrows, from an array of one known length N is a
  pointer to `[T; N]`, one cell, its address, and its `PtrMetadata` the constant N; `NonNull<T>` is a `*mut T`; a
  pointer cast to another kind of pointer, and `&(*p)`, are p; `slice::Iter` and `slice::IterMut` are a plain
  struct of their two pointers, and their own test `ptr == end` is `ptr >= end` (the pointer never passes the
  end). A reference or a raw pointer to a card's value may be an enum's field (`Option<&i16>`, with a tag) and a
  raw pointer a struct's. Then `slices::pointers_as_indexes`, after the struct pass: a family of pointer locals
  (made from, or compared with, each other) whose every definition is the address of an element of one array, a
  copy, or a step (`Offset`), and whose only other uses are `*p` and comparisons, is the elements' indexes, and
  `*p` is `t[p]`. The copies rustc's casts leave among them go, and a loop that copies the index, tests it, then
  steps it first in the body steps it last instead, so the copy is the index itself, as the for-loop pass does
  for a range.
- **Why.** At opt-level 2 rustc inlines `for x in &t`, `t.iter()` and `t.iter_mut()` into pointer arithmetic over
  a slice; all of it is code the compiler writes once a pointer is an address, but as addresses each element read
  is built from the pointer, and the end test is an equality. As indexes it is the loop `for i in 0..N` with
  `t[i]` is.
- **Cost.** None measured: `for x in &t { out(*x) }` and `iter_mut` take the cells of the same loop written with
  an index (45 and 37 cells at opt-level 2 without overflow checks), the test programs and examples unchanged. A
  slice whose length isn't known in the function (a `&[T]` parameter) is still refused, and opt-level 1, where
  rustc calls `next()`, still refuses these loops.
