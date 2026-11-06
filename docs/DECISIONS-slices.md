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

## 2026-11-05 · `for x in t` and `.enumerate()` over a slice are ranges over the indexes; `drop` is in the IR

- **Decision.** `iterators::array_into_iter`: an `array::IntoIter<T, N>` made from a whole array (rustc's
  `into_iter()` call at opt-level 1, the structs it inlines at 2) is a `Range<usize>` over its `alive` range,
  each `next()` the range's, which the for-loop pass lowers, and the item `t[i]`: t itself where nothing writes t
  again, else a copy taken where the iterator was made, as `IntoIter` takes one. `iterators::enumerate_slices`:
  an `Enumerate` of a slice iterator into one array is the same range over the slice's indexes, the item `(i -
  start, &t[i])`. `drop` is a terminator of the IR (`Terminator::Drop`), read from the driver and from the text;
  the iterator's own drop, which does nothing for elements a cell holds, becomes a `goto`, and any other drop is
  refused, as before. The cleanup unwinds of the calls in such a function are plain ones: CARDIAC never
  unwinds, and any other value whose drop did something has a `drop` of its own, which is refused.
- **Why.** Both are the loop `for i in 0..N` with `t[i]`; written as a range they take its cells (`for x in t`
  29, as the index loop; `for (i, x) in t.iter().enumerate()` 32, as the index loop), at both opt-levels for
  `for x in t`.
- **Cost.** A copy of the array (its cells, and two instructions an element) where the loop's array is written
  again after the iterator is made. `enumerate` of anything but a slice iterator, and at opt-level 1, is still
  refused.

## 2026-11-06 · An iterator's and a slice's methods that rustc calls are the loops they stand for

- **Decision.** `iterators::iterator_methods`: on a slice iterator into one array, or a range of a card's type
  built in the call's block, rustc's calls of `fold` (and `sum`, a fold with core's closure), `all`, `any` and
  `position` with a closure that captures nothing, and a range's `count`, `last` and `nth`, are counted loops
  written in the IR (`last` and a slice's `count` are no loop). The closure is called by its function's name, as
  `Option::map`'s is; the closures found by type now include those taken by `&mut self`, as an iterator's are.
  Core's sum is an overflow-checked add whatever the build's overflow checks, since the compiler isn't told them.
  `iterators::slice_methods`: `contains` and `fill` on a slice of a known array (rustc's `slice_contains` and
  `spec_fill` at opt-level 2) are loops over its elements. A closure type that names a function holding a brace
  (core's, `{closure@<i16 as Sum>::sum<..>::{closure#0}}`) is lexed to its own closing brace.
- **Why.** Each is a loop over `0..N` the compiler already compiles.
- **Cost.** A closure is a call, not inlined: `t.iter().all(|&x| x < 10)` twice is 89 cells, the same loops with
  the test written in them 53. `sum()` with overflow checks off takes the checked add's cells (39 for four
  elements, where the index loop is 25). `fold` whose result passes a card panics or keeps three digits as the
  index loop does; the support row array_iter_fold's input (3 -1 4 1 to 2941) passes one, so it reads as a
  difference now where it was refused.

## 2026-11-06 · `==` of two arrays element by element; `step_by` as core's struct, its `nth()` worked out

- **Decision.** `iterators::array_eq`: `<[T; N] as PartialEq>::eq` (and `ne`), which rustc calls at opt-level 1,
  and the compare of both arrays' bytes as one integer it makes at 2 (`copy t as u64 (Transmute)`), are the
  elements compared one after another, out at the first that differs. At opt-level 2 only for arrays assigned once
  and never written, since the integer copy is taken where rustc put it. `iterators::step_by`: `StepBy::new(r, k)`
  (and `step_by`) checks k isn't 0, as core does, and builds core's struct (the range, k - 1, and whether the
  first item is still to come) as a plain one; what rustc inlines of its `next()` stays, and the range's
  `spec_nth(n)` it calls is worked out in place: the start moved on by n where that is short of the end. Neither
  the sum nor the distance to the end fits a card in every case, so the test is split by the start's sign: from
  0 or more the distance fits, from below 0 the sum does.
- **Why.** `==` of arrays is the obvious loop; `step_by` is a loop that steps by k, and rustc's inlined `next()`
  is already code the compiler writes.
- **Cost.** `for i in (a..b).step_by(k)` is 57 cells where `while i < b { ..; i += k }` is 15 to 26: the first
  item's flag, k - 1 kept, and the split test. The while loop's `i += k` past a card is an overflow, which
  `step_by` never makes.

## 2026-11-06 · A rest pattern's subarray; a slice's `swap` and `reverse` where rustc calls them

- **Decision.** `xs[1..4]` is a projection of the IR (`Projection::Subslice`, an array's, read from the driver and
  the text), and `copy xs[1..4]`, which a rest pattern binds (`let [x, rest @ ..] = t`), is the array of those
  elements (`slices`); any other use of one is refused. A slice's `swap(i, j)` (each index checked against the
  length, as Rust does) and `reverse()`, which rustc calls at opt-level 1, are written out over the array; at
  opt-level 2 rustc inlines them into raw slices built from parts and `copy_nonoverlapping`, which the IR doesn't
  model, and they stay refused.
- **Why.** A rest binding is a few element copies; swap and reverse are the obvious loop.
- **Cost.** The rest binding's elements are copied into cells of their own.
