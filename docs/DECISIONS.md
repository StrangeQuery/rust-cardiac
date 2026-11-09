# Decision log

Newest first. Each entry is the decision, why, and what it costs. (The project was called Ferritin until it was
released on its own as rust-cardiac, was cargo-cardiac for a day, and is rust-cardiac again: the project is
rust-cardiac, and cargo-cardiac is what you install.)

## 2026-11-09 · An array of arrays is laid out by tens where it fits, with the program's other cells in its holes

- **Decision.** `nested::arrays_of_arrays` lays a `[[T; W]; H]` of scalars out by tens when W is 2 to 9 and H is
  more than 1: row r starts 10r cells after row 0, so `t[r][c]` is `t[10 * r + c]`, and 10r is one shift. The
  array is `(H - 1) * 10 + W` cells, the last row no longer than it is. The cells between one row's end and the
  next row's start are holes: the lowering builds, copies, passes and returns a grid without them, and gives them
  no initial value, so nothing in the program reads or writes them. Then `holes::fill`, on the finished listing,
  moves the program's other cells into them, as the tables-by-tens tip keeps its variables in columns 7 and 8: an
  array whole into a run of holes as long as it is, and a cell of its own (not one whose address is taken, nor one
  reached as `name+N`) into any hole left. When an index is another local plus a constant, `t[r + 1][c + 2]`, the
  element is found from `10 * r + c`, worked out once for all the neighbours, with the 12 in the instruction's
  address (`folded`), as the tip's offsets 0, 1, 2, 10, 12, 20, 21 and 22 are. A plain `r + 1` is folded only
  where r can never be a usize past a card, since `usize::MAX + 1` wraps to 0 and `10 * r` can't stand for that; a
  checked one always is. The program is compiled by tens first. That is kept when it fits in one phase, or in no
  more phases than row after row takes; otherwise the program is compiled row after row, as before. A grid 10 wide
  is by tens already, one 11 wide or more can't be, and one 1 wide is a list whose index is its row.
- **Why.** On CARDIAC the row and column of a table laid out by tens are the digits of the address. The row costs
  one `SFT 10` where row after row it is r added W times, and the holes need cost no memory when the program has
  cells to put in them. Rust can't see the gap between rows: there is no `size_of` or raw pointer arithmetic on a
  grid, and every index is checked against its own row's length first, so an index past a row's end still panics.
  A border is what the program declares: every declared row and column of one keeps its cells and its values, and
  the holes are never read as one.
- **Also.** A grid literal with the same row in it twice, as GVN leaves a bordered board's two rows of zeros
  (`[copy _2, move _3, .., copy _2]`), was refused (#2); its row is now written out each time.
- **Cost.** Cells and the program's own instructions (not the loader's), opt-level 2, without overflow checks and
  then with them, before and after. Padded alone is by tens with nothing moved into the holes.

  | Program | Before | After | Padded alone |
  |---|---|---|---|
  | array_2d_nested | 83/55, 96/68 | 72/45, 82/56 | 82, 93 |
  | array_arrays | 54/84, 54/84 | 53/83, 53/83 | 61, 61 |
  | grid_bool | 75/90, 93/112 | 75/82, 87/102 | 83, 99 |
  | grid_border | 84/226, 107 (no fit) | 73/171, 95/266 | 83, 105 |
  | grid_column_past | 64/308, 64/308 | 64/284, 64/284 | 74, 74 |
  | grid_offsets | 70/73, 91/107 | 73/65, 91/103 | 84, 103 |
  | grid_refs | 85/72, 98 (no fit) | 83/70, 96/87 | 90, 103 |
  | grid_row_past | 72/346, 72/346 | 79/330, 79/330 | 91, 91 |
  | grid_copy | 92/260, 92/260 | 92/260, 92/260 (row after row) | 119, 119 |

  Filling the holes saves 7 to 12 cells on each of these against padding alone. A grid with few other cells to put
  in its holes is bigger by tens than row after row (grid_row_past, 7 cells, for 16 fewer instructions), and kept
  so while it fits. grid_copy doesn't fit by tens (109 cells) and stays row after row. No program without an array
  of arrays changes.
- **The tip's Minesweeper count.** Four Rust versions on the tip's board of 4 rows by 6, each reading a square and
  printing how many of its neighbours are mines (tests/programs/minesweeper_*.rs). Native Rust gives the same
  count for every square from all four. None of them fits in CARDIAC's 96 cells yet, by tens or row after row, at
  any of the four settings, so they can't be run, and test_compiler.sh checks that none grows. Cells at opt-level 2
  without and with overflow checks (opt-level 1 is the same):

  | Version | Written by hand on the tip's page | Before | After | Row after row now |
  |---|---|---|---|---|
  | Packed, the edges tested with `min` and `max` | about 114, doesn't fit | 156, 172 | 146, 163 | 154, 171 |
  | A border declared, 6 by 8, a 3 by 3 loop | 88 cells, 105 instructions | refused | 149, 172 | 161, 184 |
  | A border declared, the eight neighbours written out | (the same) | refused | 175, 215 | 193, 221 |
  | A border declared, the square as one number | 78 cells, 96 instructions | refused | 144, 175 | 156, 187 |

  The page's 82-cell board by tens has no border of its own: it reads the holes as one, which a Rust program can't.
  The nearest Rust is the declared border, whose board alone is 58 cells by tens, 48 of them after the holes take
  10 other cells. Its code is what doesn't fit: 87 cells for main in the 3 by 3 version against about 25 by hand.
  The hand-written loop adds a table of offsets and trusts its input; the compiled one keeps Rust's bounds checks
  (i < 6 and j < 8 in the inner loop, 8 cells), each `for` over a range is a test of its own (about 7 cells, and 3
  more where the bound may be a usize past a card, as `inp() as usize` can be). Those are what is left (#5).
- **Evidence.** The support matrix's array_2d_nested and array_arrays match at every setting. Probes matching
  native Rust at the four settings, each ending in a bounds failure: a bool grid with an index changed between
  uses; a row of squares inside a border the program declares, of different values above, at the sides and below,
  so a sum shows each border cell it reads; a row read past its end, whose next cell by tens is a hole; a u8 grid
  wrapping; a grid copied whole and changed; constant offsets as the row and column change in a loop, and one from
  usize::MAX that wraps to 0; grids passed by `&` and `&mut`; and grids 10, 1 and 11 wide.

## 2026-11-09 · A check made again is a goto, and a passed check shows a usize is on a card

- **Decision.** `blocks::repeated_checks`, before the range pass and again after the merges of repeated work: an
  assert whose only way in comes straight from a block that asserted the same thing, expecting the same, with
  nothing it reads set in between, is a goto. The same thing is the same bool, or a bool each block works out from
  the same comparison of the same locals. The lowering fuses a comparison into its assert when the assert copies
  its bool too, if nothing else reads it. And a usize that may be past a card is not tested for its sign again in
  a block whose only way in is a passed `x < k` (or one that came from such a block), until something sets x.
- **Why.** rustc's GVN asserts one `r < 6` for each `b[r][..]` that indexes by the same r, as `assert(copy _20)`
  in each block, and the lowering stored each such bool through a branch and tested it again: 13 cells a check
  (#3). After `assert(r < 6)`, r is 0 to 5, but `CLA r / TAC rt_panic` came back before each use (#4).
- **Cost.** Nothing: fewer cells and instructions. By tens, main of the eight-neighbour Minesweeper count went from
  154 cells of code to 114.
- **Evidence.** The test suites, the examples and the support matrix, at every setting.

## 2026-11-07 · A value past a card read again after its store is what the cell keeps

- **Decision.** Without overflow checks, a sum, difference or product that may pass a card leaves the whole value
  in the accumulator when it is stored, and the cell keeps the last three digits. Its store is marked, and the
  peephole rules that drop a `CLA x` because the accumulator already holds x (2, 8, 10, 13 and 14) don't count a
  marked store: a read after it loads the cell. A store is marked when the range analysis, run again on the code
  as it is lowered, can't show the result stays on a card (an op the range pass showed stays on a card, written
  once, keeps that), and the value is read in more than one place. The range pass also takes a value past a card
  for what its cell keeps after the statement that follows it, not the whole result.
- **Why.** `let p = a.wrapping_mul(40)` with a = 30 printed 200 and then `p > 500` was true: the comparison read
  the 1200 the store had left in the accumulator. And with a known from 25 to 30, the range pass took p for 1000
  to 1200 and decided `p > 500` with no test, so p = 40 was more than 500. One value now reads one way. A value read
  in one place only still may be read exact there, as a carried one is: no reading of it disagrees with another,
  and `(month - 1) as usize` for month = -999 still panics, as natively.
- **Cost.** One `CLA` for each marked store a read follows; nothing where the analysis shows the value stays on a
  card. Of the examples, voxel and the ray tracer's hop cost 2 and 3 cells, guess, powers, floor, span and hit 1.
  The analysis runs once more per function.

## 2026-11-06 · A multiply by a negative constant takes the product away from 0

- **Decision.** `x * -k`, for a constant k that isn't the routine's, is x taken away k times from 0 (with the decimal
  shifts in between, as for `x * k`), not `x * k` stored and then taken from 0. A value carried in the accumulator
  for it goes to its cell first.
- **Why.** Stored, a product past a card kept three digits before anything read it: with overflow checks on,
  `999 * -9` gave -991 and `99 * -19` gave -881 where they panic (the check saw what was left), and an `i8`'s
  `wrapping_mul(-39)` wrapped those three digits, not the product: `-128i8.wrapping_mul(-39)` was -32, not -128.
  Taken from 0, the whole product is in the accumulator, as it is for a positive constant.
- **Cost.** Two instructions fewer for each: the product's store and its reload go, and a `CLA 0` comes first.

## 2026-11-06 · A wrapping `*` past a card is what CARDIAC keeps of it

Supersedes "A wrapping `*` is refused where the product can pass the accumulator" (2026-11-06), on Jason's ruling:
the machine has a standard, and we don't get to choose.

- **Decision.** A `u8`'s or an `i8`'s wrapping `*` (overflow checks off, or `wrapping_mul` inlined) that the range
  pass hasn't shown stays in its type's range compiles, whatever its factors. By a constant from -39 to 39 it is
  as before: the product is whole in the accumulator (9999 at most) and wraps as Rust's does. Any other is the
  multiply routine's, the constant as the count of adds: where the product passes a card, what is left of it is
  CARDIAC's own rule for a value past a card, the rule an `i16` has without overflow checks: the cell keeps its
  last three digits, with its sign, as 999 + 1 stores 000. The type's ordinary wrap is then taken of those three
  digits, as of any value on a card, so the result is always in the type's range: `200u8.wrapping_mul(200)` is
  40000, stored 000, so 0 (Rust: 64); `255u8.wrapping_mul(255)` is 65025, stored 025, so 25 (Rust: 1); `-128i8 *
  100` is -12800, stored -800, so -32 (Rust: 0). A product from -999 to 999 wraps exactly as Rust's does. Both
  factors known (rustc folds these, but a pass of ours could leave one) is Rust's own wrapped product.
- **Why.** CARDIAC has no multiply: a product is a loop of adds, and past a card the machine's overflow rule
  decides it, as the manual and the reference simulators have it (ADD and SUB in a four-digit accumulator, STO keeping
  the last three digits with the sign: Brian Stuart's simulator stores ndig(val, 3), CardiacVisualizer
  accumulator % 1000, burn wraps the accumulator at 10000). The routine stores its partial product after every
  add, each a card's three digits plus a factor, so the accumulator never passes four digits and the three digits
  left don't depend on how a machine bounds its accumulator: burn and a simulator that doesn't bound it give the
  same. The routine, not adds and shifts in place, for a big constant: in place the accumulator would hold more of
  the product than a card, and what the program later did with it would depend on whether it was read from the
  accumulator or from its cell. Jason: "Somebody somewhere will exploit that behavior for fun and profit. Let
  'em."
- **Cost.** Nothing for what compiled before, and overflow-checked builds are unchanged: there a `u8` product past
  255 panics, as Rust's does. The routine is slow for a big count (up to 255 passes of its loop), which only code
  that was refused before pays.

## 2026-11-06 · `cargo cardiac help` takes a command or a code, as `go help` and `git help` take topics

- **Decision.** `help` is the one command that explains: `help` alone is the usage (as `--help`), `help COMMAND`
  that command's part of it (for `debug` and `asm`, what ekg and carc say to `--help`), and `help CODE` a code's
  explanation, as `explain CODE` printed it, then the code's page on a line of its own, which terminals make a
  link. A word that is neither is a plain error; one of a code's shape that isn't one says so. `help --json` is
  every code, for the website's pages. `explain` stays as a quiet alias (`explain CODE`, `explain --json`), out of
  the usage, so the commands people and the site's build have used keep working. The line after a failed build
  says `cargo cardiac help RCE0013`. `explain --open` is gone: printing the address gives the same in one click,
  without a command that starts a browser; `view` still opens Cardigan.
- **Why.** rustc's own split (`--help` and `--explain`) is two words for one question, "what does this mean";
  `go help TOPIC` and `git help` answer it with one, and a code is one more topic.
- **Cost.** A command name and a code can't collide, as a code is RC, a letter and four digits. The usage text
  is built from each command's part, so `help COMMAND` and the usage can't drift apart.

## 2026-11-06 · A `&dyn Fn(..)` of closures that capture is an enum of their structs

- **Decision.** The IR reads a closure trait's arguments, `Fn(i16) -> i16`, as rustc has them,
  `Fn<(i16,), Output = i16>` (`GenericArg::Binding` for the output), and prints them back as rustc writes them. A
  closure coerced to `&dyn Fn(..)` (or `FnMut`, `FnOnce`, through a shared reference) is a variant of the object's
  enum (`dyn_Fn_i16_i16::closure0`), as a struct is (the entry on `&dyn Trait`), its trait known by its last segment
  however rustc trims the path. A call through the object is, in each arm, the closure's own call through its
  trait, `<{closure@..} as Fn<(i16,)>>::call`, which the closure pass then makes a call of its function with its
  struct; where there are several arms, the tuple of arguments rustc built for the call is built again in each, beside
  its call. After the closure pass, a variant holding a closure holds its struct (`main_closure0_env`).
- **Why.** A `&dyn Fn` is a trait object whose types are closures, and each closure is a type the program names
  where it makes the object, so the same switch to direct calls works; the closure pass already turns a closure's
  call through its trait into a call of its function.
- **Refused.** A closure that captures nothing behind `&dyn Fn` (its variant would hold no value, which the enum pass
  doesn't take: a function pointer does the same job), `&mut dyn FnMut`, `Box<dyn Fn>`.
- **Cost.** None to programs without them. A closure's struct isn't taken apart into its captures for the object's
  variant, so two closures' captures don't share cells.
- **Evidence.** Support matrix: closure_passed_dyn matches at every setting (42 cells, 53 with overflow checks).
  Probes matching native Rust at the four settings: two closures chosen at run time behind one `&dyn Fn`, a
  `move` closure passed down two functions as `&dyn Fn() -> i16`, a closure of two parameters returning a bool made
  anew in a loop.

## 2026-11-06 · A trait object of a struct of scalars holds the struct's fields

- **Decision.** The driver gives each struct's fields, in order, by name and type (`StructDef::fields`; a name two
  structs share, or a generic struct's instances, gives none). A `&dyn Trait` made of a struct of the program's whose
  fields are all scalars is a variant of those fields, `dyn_Shape::Rect(i16, i16)`, not of the struct; a method
  called through it is called on the struct built again from them (`_5 = Rect { 0: .., 1: .. }; _2 = &_5`), which the
  struct pass takes apart.
- **Why.** The enum pass shares cells between variants only by type, and two structs are two types, so an object of
  `Sq(i16)` or `Rect(i16, i16)` was a tag and three cells; with the fields as the variants', it is a tag and two, and
  passing it passes three. dyn_choose: 98 cells to 89 without overflow checks.
- **Cost.** None found: the struct built again costs nothing once copies are forwarded. A struct with a field that
  isn't a scalar (another struct, an array) is held whole, as before.
- **Evidence.** Support matrix: dyn_choose matches without overflow checks; with them it is 117 cells (the two
  checked multiplies), and stays too big. Probes matching native Rust: objects of a named struct of two i16s and of
  a bool and an i16, returned from a function; a struct with a struct field beside a tuple struct.

## 2026-11-06 · `max` and `min` on a struct whose `Ord` is derived; a `Cmp` only compared with 0 is the comparison

- **Decision.** The driver makes core's `Ord::max` and `Ord::min` instances for one of the program's structs whose
  `PartialOrd` is derived and whose fields are integers, bools or chars, as it does for C-like enums. The `<` such a
  body calls through `PartialOrd` (`lt`, `le`, `gt`, `ge`, core's own defaults) is written out as the derived
  `partial_cmp` decides it: each field but the last tested both ways (`a.k < b.k` true, `b.k < a.k` false, for `lt`
  and `le`), the last compared with the operator itself, each answer a constant the compiler takes straight to its
  test. And in the front end, `d = Cmp(a, b); x = discriminant(d); t = Lt(move x, const 0)` in one block, where
  nothing else names d or x and nothing in between writes a or b, is `t = Lt(a, b)` (and so for the other
  comparisons): `Less`, `Equal` and `Greater` are -1, 0 and 1.
- **Why.** core's generic `max` calls `PartialOrd::lt`, whose default reaches the derived `partial_cmp` through an
  `Option<Ordering>`: as calls, an instance of `lt` and the derived function, it was over 98 cells. The `Cmp` is what
  rustc leaves where it inlines a `<` of a `partial_cmp` written by hand over integers
  (`self.0.abs().partial_cmp(&o.0.abs())`); as an `Ordering` it was two tests and three stores before the test of it.
- **Cost.** None: the compiler's tests and the examples are unchanged.
- **Evidence.** Support matrix: derive_ord_max matches at every setting (85 cells); trait_partialord_by_hand now
  fits at opt-level 2 without overflow checks too (86). Probes matching native Rust: `<=`, `>`, `<` and `>=` through a
  generic function on a struct of an i16 and a bool, `min` on a struct of a u8 and two i16s, negative values and
  equal first fields; a struct with a struct field stays refused.

## 2026-11-06 · An iterator adaptor chain is the loop it stands for, read back from where its structs are built

- **Decision.** `adaptors::adaptor_chains`, beside `iterator_methods`: where rustc calls what consumes a chain of
  core's adaptors (`<Filter<..> as Iterator>::count`, or, where it inlined that, the root iterator's `fold` with a
  closure of core's for each adaptor, `map_fold(f, filter_fold(p, sum))`), the chain is read back from the lines
  that built it: a range, a slice iterator into one array, `once`, `repeat`, `successors` and `step_by`, then
  `map`, `filter`, `enumerate`, `skip`, `take`, `rev`, `chain` and `zip` (two sources of no closures of their
  own), from their structs, their calls, and the types of core's closures (`ir::parse_ty` of the text the IR keeps
  for a closure's type, so each is a typed path whose arguments name the closures it wraps). The consumer (`sum`,
  `count`, `fold`, `rfold`, `for_each`, `max` and `min`) is then written as one loop over the source, each item taken through the adaptors in core's order (`skip`
  takes the first n through the adaptors below it; `take` stops at the nth without asking for another;
  `successors` works out the next item before giving this one; `zip` asks the second source after each of the
  first's items). What a struct holds (a range's ends, `skip`'s n, a closure's struct) is copied before the line
  that built it, so the loop reads what the chain was built with, however often rustc's MIR reads one value of a
  chain again. A closure of the program's is called as its function, given a reference to its struct if it
  captures; a closure of core's is matched to what it captures by the program's closure among its fields (whose
  type names no path, as core's printed in full and trimmed differ). `t.map(f)` on an array (core's `try_map`
  with `f` wrapped never to stop) is the loop over its elements (`adaptors::array_maps`).
- **Considered.** Taking the adaptor structs apart in the struct pass, with each adaptor's `next()` written out:
  that is the shape rustc leaves in a `for` loop, but where it calls the consumer the consumer is core's code,
  generic over closures, which isn't here, so the consumer would still have to be written; and a `next()` per
  adaptor is a test and an `Option` per adaptor per item, where the loop is one test per item.
- **Closures are inlined.** A closure the loop calls whose body is plain code (no calls, nothing the IR keeps as
  text) is copied in, its locals its own and its struct read where the copy is: a closure here is called from one
  place (two, in a `chain`), so the copy costs nothing and saves the call. `.map(|&x| 10 - x).max()` and `.min()`
  together (the support row iter_map_max) are 78 cells this way, 93 with overflow checks (with two calls, 97 and
  110: too big). Only the closures this pass calls.
- **Also.** A load left with nothing to read it once the compiled stack has shared a call's result cell with the
  local it was copied into (`CLA f_0` then `STO x`, the store gone) goes too: `add_times_count` runs 4
  instructions fewer. A tuple of cells and references to cells (`(usize, &i16)`, enumerate's and zip's items) is read as a
  function's parameter and taken apart as a struct; a closure of core's of all zero bytes
  (`{transmute(0x0000000000000000): {closure@..enumerate..}}`, enumerate's count of 0) is a constant of the IR
  (`Const::Zeroed`); a closure of core's built of what it captures is kept for this pass, and refused after it.
- **Refused.** An adaptor not here (`filter_map`, `flat_map`, `peekable`, `scan`, `take_while`, `chunks` of an
  iterator, ...); a chain held and stepped by hand, or in a `for` loop (where rustc inlines `next()` into the
  structs); `rev` of `enumerate`, `skip`, `take`, `zip`, `step_by` or `successors`; `zip` whose first iterator
  calls a closure (core's `zip` of iterators with random access gives up a different item past the shorter's end
  than the plain one, which this can't tell apart), or whose second isn't a range or a slice's iterator; `any`,
  `all`, `find` and `position` on a chain, which rustc inlines into `try_fold` with closures of core's that hold a
  `ControlFlow` (on a plain slice iterator or range, `iterator_methods` writes them, as before).
- **Cost.** None to programs without chains. A filter's closure on a slice's items is given `&&T`, read through
  two addresses. A sum is core's overflow-checked add whatever the build's checks, as `iterator_methods`'s is.
- **Evidence.** The support matrix's 16 iterator-adaptor rows match at every setting (iter_map_sum 44 cells,
  array_iter_count 38, iter_zip 59), 0 rows worse. Probes matching native Rust at
  the four settings, on negative values, empty ranges, `take(0)`, `skip` past the end, uneven `zip`s: `skip` and
  `take` in both orders, `rev().fold`, `chain` then `rev`, `chain` with `filter` and `count`, `zip` of a range and
  a slice and of a reversed slice, `enumerate` then `map`, `filter().max()` and `.min()` of a slice's
  items (also with none passing),
  `repeat(x).take(n).enumerate()`, `once(a).chain(once(b))`, `step_by` with negative starts and a step past the
  end, `for_each`, closures that call `out()` in `map` and `filter` under a `take` (the order of their calls), an
  `FnMut` closure that counts through a captured `&mut`, and `move` closures that capture by value.

## 2026-11-06 · `core::array::from_fn(f)` is the loop `t[i] = f(i)`

- **Decision.** `closures::array_from_fns`, before the closures are made structs: rustc inlines `from_fn` into a
  call of core's `try_from_fn` given core's closure that wraps f (`NeverShortCircuit::wrap_mut_1`), and reads the
  result, a `NeverShortCircuit<[T; N]>`, as its field `.0`. That is the loop core's makes: for i from 0 up to N,
  `t[i] = f(i)`, f called through `FnMut` as rustc calls a closure (which the closure pass then makes a call of
  its function, given its captures), and the result is the array itself. Only where the wrapper is built in the
  call's block of f alone and the result is read only as its field.
- **Why.** core's generic body is never compiled; what it does is a counted loop of calls.
- **Cost.** None to other programs. f a closure that captures nothing (passed as a constant) stays refused.
- **Evidence.** array_from_fn matches at every setting (76 and 87 cells). Probes matching native Rust at the four
  settings: a closure that outputs i as it is called (in order), one making a `[bool; 4]`, and one that panics
  part-way through.

## 2026-11-06 · `Option::get_or_insert` is core's test and a `&mut` of the value

- **Decision.** `enum_methods::gets_or_inserts`: `o.get_or_insert(v)`, which rustc calls as `get_or_insert_with`
  given core's own closure (one that returns the value it captured, built just before in the same block), is
  core's `if let None = *o { *o = Some(v) }` and then `&mut` of the value in `Some`; the closure's line keeps v in
  a local of its own. The closure pass leaves that closure for it, where it made any other closure it can't take
  apart a refusal.
- **Why.** A `&mut` of a variant's field is what the enum pass already does for `if let Some(x) = &mut o`.
- **Cost.** None to other programs. A value rustc folds into the closure as a constant (`get_or_insert(200)`,
  `const {transmute(0xc8): {closure@..}}`) stays refused.
- **Evidence.** option_get_or_insert matches at every setting (42 and 54 cells). A probe with an `Option<i16>`
  that was and wasn't `Some`, the value changed after the call, written through the `&mut`, and an `Option<u8>`'s
  added to, matches native Rust at the four settings.

## 2026-11-06 · `NonZero<T>` is its integer, and `Option<NonZero<T>>` an `Option<T>` whose `None` is 0

- **Decision.** `nonzero::nonzero_is_its_value`, in the front end: in a function whose transmutes of a NonZero are
  the ones rustc writes for `NonZero::new` and `get` (`x as Option<NonZero<T>>`, `n as T`, `x as NonZero<T>`), every
  `NonZero<T>` of an integer is a T, its transmutes to and from T copies, and `x as Option<NonZero<T>>` is the test
  Rust's layout makes: `None` where x is 0, else `Some(x)`, an `Option<T>` (the enum's definition normalized to
  it). A type named alone (`NonZero<T>`, `Reverse<T>`, as rustc trims core's paths) is core's only where the
  program has no struct or enum of that name, which the front end is now given.
- **Why.** A NonZero holds its value; only its `Option`'s layout is special, and that is one test.
- **Cost.** A test of 0 where rustc's transmute is none. Any other transmute of one leaves the function refused.
- **Evidence.** option_nonzero matches at every setting (16 and 18 cells). Probes matching native Rust at the four
  settings: a `NonZeroI16` passed to a function and its `get()` doubled, `NonZeroU8::new(..).is_some()`, `unwrap()`
  (panicking at 0), `map(|n| n.get() + 1)`, and `==` of two `Option<NonZeroI16>`s; a program's own `Reverse<T>`
  is its own.

## 2026-11-06 · `Reverse(x)` of an integer is x, compared the other way round

- **Decision.** `ordering::reverse_is_its_value`, in the front end before the three-way compares: in a function
  whose parameters and result hold no `core::cmp::Reverse`, a `Reverse<T>` of an integer T (or a reference to
  one) is a T, `Reverse(x)` is x and `.0` the value; `<Reverse<T> as PartialOrd>::lt(&a, &b)` and its kin, and
  `Ord::cmp`, are T's with the two arguments swapped; `Ord::max` (and `core::cmp::max`) is T's `min`, and `min`
  T's `max`. Where anything else names `Reverse`, the function is left as it was, refused.
- **Why.** `Reverse` is a transparent wrapper whose comparisons swap their operands; for an integer, two equal
  values are the same value, so which of two equals `max` returns can't be told.
- **Cost.** None to other programs.
- **Evidence.** cmp_reverse matches at every setting (28 cells). `cmp`, `>=`, `core::cmp::min`, `max` of two
  `Reverse<i16>`s and `<=` of two `Reverse<u8>`s, for a below, equal to and above b, match native Rust at the four
  settings.

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

## 2026-11-06 · A function that calls itself has a stack of its own, as deep as the memory left over

- **Decision.** `call_stack::software_stacks`, after the passes that take structs and enums apart and before the
  call graph is checked: a function that calls itself other than at its end (`n + sum(n - 1)`) calls itself no
  more. Each such call is a check that the stack has room, the locals still read after the call (by liveness,
  the result left out) pushed, each onto an array `[T; depth]` at the stack's count, the arguments into the
  parameters, the count one more, and a jump to the function's start; each return, when the count isn't 0, is the
  count one less, the result copied into the call's place, the locals popped, and on after the call. Where it calls
  itself in more than one place, a `[u8; depth]` says which place to go back to. The calls share the arrays: a
  call's first local of a type is on the first array of that type, and so on. A call with no room panics, `stack
  overflow`, before it writes anything. The depth is what memory holds: the program is compiled with stacks one
  call deep, then again with the cells left over shared out among the arrays (one cell a call each), one call less
  until it fits. A function with nothing to push keeps only the count, which may go to 999.
- **Why.** CARDIAC has no stack and each function's cells are fixed, so the call inside writes over what its
  caller still reads. A loop with an explicit stack is code every pass after it already compiles well (the
  arrays are indexed reads and writes, through built addresses), and the rest of the program is unchanged. Rust
  panics on a stack it overflows too, only much deeper.
- **Refused, as recursion is.** A function with a reference or a pointer among its locals or parameters, or a local
  whose address is taken (either could point into the cells being reused); one that returns a struct (a result in
  cells of its own) or reads one's; a local still read after the call that isn't a one-cell number or a bool;
  recursion through more than one function, unless every call in it is at its end (the entry before).
- **Cost.** About 30 cells of code for one place it calls itself, and a cell a call for each local it pushes.
  `sum` (support row fn_recursion_direct) is 59 cells with a stack one call deep, so its stack is 37 calls deep at
  opt-level 2 (26 with overflow checks at opt-level 1). Fibonacci (two calls, a stack of each `n` and the first
  result and one of where to go back) is 104 cells with a stack one call deep: too big. Programs that don't call
  themselves are unchanged and compiled once, as before.
- **Evidence.** fn_recursion_direct matches at every setting. Probes matching native Rust at the four settings,
  where the recursion fits the stack: `sum` called twice with its argument read after; `down(n)` printing on the
  way down and up (a `()` result, nothing pushed); Ackermann (a call whose argument is a call: two places); a
  struct parameter; a u8 recursion; a local computed before the call printed after it. Deeper inputs panic with
  `stack overflow`. A `&mut` parameter, a struct result and an `Option` result are refused. The test program of
  recursion refused (`tests/programs/recursive.rs`) now compiles, so it is recursion through two functions.

## 2026-11-06 · Functions that call one another at their ends are one function, with a start for each

- **Decision.** In the front end, before a function's call of itself at its end is a loop, `cycles::mutual_tail_calls`
  finds each cycle of the program's functions (`is_even` calling `is_odd(n - 1)`, which calls `is_even(n - 1)`) in
  which every call of one of the cycle by another is a call whose result is the caller's own, with nothing after it
  but the way to the return. Its functions are one: each one's blocks, its entry a start of its own, and each such
  call its arguments into temporaries, the temporaries into the parameters of the one it calls, and a jump to that
  one's start, as `front::tail_calls` does for a call of itself. When the rest of the program calls only one of
  them, the merged function is that one, under its name, its parameters its own, with the others' code in it; the
  others go. When it calls more than one, the merged function is named after them all (`is_even_is_odd`), its first
  parameter, a u8, says which was called, and its entry is a `switchInt` on it to that one's start; every call of
  one of them from outside the cycle is a call of it, with that number. A parameter is shared by the functions'
  parameters of its type, so `is_even(n)` and `is_odd(n)` have one `n`; a call passes a zero for a parameter of
  another function, so where that one isn't a number or a bool the cycle is left as it is.
- **Why.** No call is left inside the cycle, so each function's cells hold one round's values at a time, as a loop's
  do: it needs no stack. The merged function is an ordinary one to every pass after the front end.
- **Refused, as recursion is.** A cycle with any call of one of its functions that isn't at the caller's end, whose
  functions return different types, or with a reference or a pointer among their parameters or the arguments of
  the calls between them (which could point into cells being reused); a cycle through `main`, or through a
  function that isn't one of the program's of a plain name (a method).
- **Cost.** None to programs without such a cycle: the compiler's tests and the examples are unchanged. With
  more than one way in, a call from outside passes the number (two cells) and the entry tests it.
- **Evidence.** Support matrix: fn_recursion_mutual matches at every setting (38 cells). Probes matching native Rust
  at the four settings: `is_even` and `is_odd` both called from main (48 cells); three functions of different
  parameters (`a(n, acc)`, `b(big: bool, n, acc)`, `c(m)`), each called from main; a struct parameter; an
  `Option<i16>` result; a member with a loop that changes its parameter; one that also calls itself at its end and
  calls a function outside the cycle; the two called through an array of function pointers; the merged name
  taken by a function of the program. A cycle through a reference parameter, and one whose call isn't at its end
  (`1 + g(n - 1)`), are refused.

## 2026-11-06 · Every message about a program has a code, explained on the web and by `cargo cardiac explain`

- **Decision.** What cargo-cardiac says about a program carries a code, as rustc's errors do:
  `error[RCE0013]: src/main.rs: recursion (main -> f -> f) needs a stack; this backend has none yet`, with
  `warning[RCW....]` for a warning and `note[RCI....]` for a note. Rust numbers only its errors and names its
  warnings as lints; we number all three, because on CARDIAC a warning or a note (a bool better as a `Flag`, an
  expensive operation) needs as much explaining as a refusal. The message stays one line; the explanation (what it
  means on CARDIAC, why, what to write instead, an example) is Markdown in the code's entry of one table,
  `src/codes.rs`, which `cargo cardiac explain CODE` prints, as `rustc --explain` does, and which the website's
  pages, `/rust-cardiac/messages/` and `/rust-cardiac/messages/CODE/`, are made from (`cargo cardiac explain --json`,
  committed in the site's repository). Each message is a constructor in its code's entry, so a message can't be made
  without one: the compiler's `Fail` is a `codes::Diag` (its code, its line, its notes), not a `String`, and travels
  from the driver to `cargo cardiac` as JSON. A code is what went wrong, not where: 48 codes cover about 170 places.
  In `--message-format=json` the code is the message's `code`, and a note under an error is in its `children`. A
  usage error and the tool's own failures (no driver, a file it can't write) have no code (`Diag::tool`) and stay
  plain `error:` lines. A program that uses `&`, `|` or `^` between values, a shift by a variable or the bit
  methods gets one warning, RCW0001, naming them (`expensive operation, consider an alternative: \`&\`,
  \`count_ones()\``), also under the error when it is too big; `*` and `/` of two variables don't.
- **Why.** The messages had grown into paragraphs (the dev profile's note was five lines). A code keeps the line
  short and gives the reason a place to live, searchable, the same offline and on the web.
- **Cost.** A code is never renumbered or reused. The old per-word check of which errors `explain` may restate
  in Rust's terms (`println!`, `Vec`) is a check of the code, so a few messages that happened to contain
  "unsupported" (a struct copy) are now said as they are. The dev profile's note and the generic-function error
  are shorter, their reasons in the explanation. No listing or test outcome changed.

## 2026-11-06 · A `&str` is the slice of its bytes

- **Decision.** `front::strings_as_bytes`, before byte strings are built: a `&str` local, parameter or result is
  a `&[u8]`, and a string literal it is set to (or `as_bytes()` of one) is the byte string of its bytes, which the
  slice passes then lay out as any other. Only literals of ASCII and the plain escapes; any other stays refused.
- **Why.** `.len()` of a literal, its bytes indexed, and a `&str` chosen at run time are slice operations the
  compiler now has.
- **Cost.** A literal is built as an array where it is used, two instructions a byte: `==` of two `&str`s is
  still refused (core's `compare_bytes`), and would be too big as built.

## 2026-11-06 · A function that returns a slice returns its length in a cell of its own

- **Decision.** `run_time_lengths`: a function whose result is a slice keeps the slice's length in a usize of its
  own beside its result, one of its `results` (as a struct's fields are), and each call of it reads the length
  from that cell on the way back (`@f.n`, as the caller reads a struct's). A function whose result's length
  can't be followed is refused.
- **Why.** `fn tail(s: &[i16], k: usize) -> &[i16] { &s[k..] }` was refused; a `&str` a function returns is one.
- **Cost.** A cell in the callee, and a copy at each call.

## 2026-11-06 · `find`, `max` and `min` on an array's iterator; a reference to a reference is a cell

- **Decision.** `iterators::iterator_methods` writes three more of the calls rustc leaves on a slice iterator
  into one array: `find` with a closure that captures nothing (the closure given a reference to the item, as
  core gives it), and `reduce` with the closures core's `max` and `min` pass, as a loop keeping the index of the
  greatest (the later of equals, as `max_by` keeps) or the least (the earlier). A reference to a cell that holds a
  reference to a card's value (`&&i16`, what `find`'s closure takes) is one cell, its address, read through twice.
- **Why.** Each is the loop `for i in 0..N`; the result is `Option<&T>`, which the enum pass already holds.
- **Cost.** None found. `find` on a closure that captures, and these on a slice of run-time length, stay refused.

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

## 2026-11-06 · A closure that captures is a struct of its captures; a call of it through `Fn` is a call of its function

- **Decision.** The front end makes each closure that captures a struct (`closures::closures_as_structs`), named
  after its function (`main_closure0_env`), built where rustc builds the closure (`{closure@..} { k: copy _5 }`,
  which the IR now reads as `Rvalue::Closure`) and taken apart by the struct pass as any struct. A call of a closure
  through its trait, `<{closure@..} as Fn<(i16,)>>::call(move _3, move _4)` (or `FnMut`'s `call_mut`), is a call of
  its function with the struct and the arguments rustc tupled, one by one: a constant tuple's values, or the
  operands of the tuple the block built just for the call. With that, a closure passed to one of the program's
  generic functions is the generic function's instance for that closure's type (the entry on instances), a closure
  returned as `impl Fn` is the struct returned, and one kept in a generic struct is a field of that struct's
  instance. The IR also reads what these need and the parser didn't: a tuple of one (`(copy _2,)`, and the constant
  `(10_i16,)`) and a copy without a retag (`no_retag copy ((*_1).0: &i16)`, which is the copy).
- **Why.** Each closure is a type of its own, so rustc writes the code that takes one generically, through `Fn`,
  and the instance machinery already makes a function of each instance; what was missing was the closure as a value
  (its captures) and the call it makes through `Fn` (its function). A capture by reference is a reference field, an
  address, as a slice iterator's pointers are; by value, the value.
- **Refused, where the function that does it is compiled.** A closure called through `FnOnce::call_once`, or whose
  function takes it by value; one that captures a field of a struct (`p.a`, which rustc captures as a place the
  struct pass doesn't take apart); a closure defined in a module's function or a generic one; a tuple of one left
  over; core's generic functions given a closure (`array::from_fn`, whose MIR is core's: the linker refuses it).
- **Cost.** None to programs without such closures: the compiler's tests and the examples are unchanged.
- **Evidence.** Support matrix: closure_passed, closure_returned, closure_stored and closure_generic_arg match at
  every setting; and three rows the copy without a retag was all they lacked (array_get, option_as_ref,
  option_copied). Probes matching native Rust at the four settings: an `FnMut` that adds to a captured variable,
  called twice, and one called through `&mut F`; a `move` closure whose variable changes after it is made; a
  closure capturing an array and indexing it; a closure passed down from one generic function to another by
  reference; a closure made anew in each pass of a loop around a changing variable.

## 2026-11-06 · A `&dyn Trait` is an enum of the types the program makes one of; a method call through it a `match`

- **Decision.** The front end makes each trait object an enum (`indirect::trait_objects`): for a trait the program
  makes `&dyn Trait`s of, `dyn_Shape`, with a variant for each of the program's structs a reference to which is
  coerced to one (`copy _11 as &dyn Shape (PointerCoercion(Unsize, Implicit))`), holding the struct's value. The
  coercion builds the variant (`dyn_Shape::Sq(copy (*_11))`), a reborrow (`&(*_1)`) is a copy, and a call of a
  method through the object, `<dyn Shape as Shape>::area(move _1)`, is what rustc writes for a `match`: its
  discriminant, a `switchInt` (the last type the otherwise), and in each arm the type's own method called on a
  reference to its variant's value (`<Sq as Shape>::area`). The enums go to the enum pass with the driver's, so an
  object is passed, copied and returned as any enum is. The driver names each type's methods of the trait as a call
  names them, as it does the functions a pointer is made to.
- **Why.** As with function pointers, every type an object can be is named where rustc's MIR makes it, so a call
  through its table of functions is a choice among calls known at compile time. A shared reference can't see its
  value change while it lives, so the object can hold the value, as a shared reference to a struct already is the
  struct (`structs::shared_refs_by_value`); and with the object an enum, everything the enum pass does for enums
  (tag cells, sign tags, values passed and returned) is done for objects too.
- **Refused, where the function that does it is compiled.** An object of anything but one of the program's structs
  of a plain name (a primitive's impl, a generic struct's, a closure: `&dyn Fn`); a trait with generic arguments;
  `&mut dyn Trait`; a method the impl doesn't define (the trait's default, which would be an instance of a generic
  function), or one of a trait the object's trait extends; two of the object's types of the same name; anything
  else done through an object (a read through its pointer, a cast). Their error says what works.
- **Cost.** A variant's value is its struct's cells, which the variants don't share (the enum pass shares a cell
  only between fields of one type): an object of `Sq(i16)` or `Rect(i16, i16)` is a tag and three cells, and
  passing one passes all four. dyn_choose, whose two `area`s multiply, is 98 cells, two over.
- **Evidence.** Support matrix: method_dyn, trait_dyn_dispatch, trait_dyn_single and dyn_one match at every
  setting; dyn_choose is too big; closure_passed_dyn stays refused (a capturing closure behind `&dyn Fn`). Probes:
  an object returned from a function, objects of two traits with a method of one name, and an object passed down
  two functions match native Rust at the four settings; a default method, a supertrait's method, an `impl Trait for
  i16` and an impl for a generic struct are refused.

## 2026-11-06 · A function pointer is a number; a call through one is a switch to a call of each function it can be

- **Decision.** The front end makes every function pointer a number (`indirect::function_pointers`): the
  function's place among those the program makes pointers of that type to, in the order it first makes them, held
  as a `u8`. `double as fn(i16) -> i16 (PointerCoercion(ReifyFnPointer(Safe), ..))` is `const 0_u8`; a closure that
  captures nothing, coerced (`ClosureFnPointer`), is its function's number; and a call through a pointer, `_7 =
  copy _8(move _10)`, is a `switchInt` on the number to a call of each function of that type, the last the
  otherwise, or the call itself when there is one. A pointer, or an array of them, may be made, copied, filled,
  indexed, borrowed, passed and returned. The IR has what it takes: a function pointer's type (`Ty::FnPtr`, `fn(i16)
  -> i16`, a `for<'a>` binder's lifetimes left out), a function as a value (`Operand::Fn`), the two coercions as
  cast kinds, a closure's zero-sized value with its type (`Const::ZeroSized`), and the call through a pointer
  (`Terminator::CallIndirect`), which no pass after the front end meets. The driver reads them all, and names an
  impl's function a pointer is made to as a call names it; the linker renames a pointer's function as it renames a
  call's.
- **Why.** Every function a pointer can point to is named where rustc's MIR makes the pointer, so the set is fixed
  at compile time. CARDIAC's indirect jump is one the program builds itself (as an indexed read is), but a jump to
  the address in a cell isn't a call: each function has cells of its own for its parameters and its result, so the
  caller has to know which function it calls to pass it anything. A switch to direct calls keeps every pass after
  it (parameters passed on, cells shared by liveness, recursion found through the calls) as it is for any call.
- **Considered.** The pointer as the function's address and a call as a built `JMP` to it: every function a pointer
  can be would have to share its parameters' and its result's cells, which the cell allocator has no way to say.
  A jump table for the switch (a computed jump into a row of calls): for two or three functions the test chain
  is smaller (two cells a function), and a switch with many dense arms could get one for every `match`, not only
  here.
- **Refused, where the function that does it is compiled.** Anything else done with a pointer: compared (Rust says
  the result isn't meaningful), cast to an address, kept in a struct, a tuple, an enum (`Option<fn(..)>`), a
  `const` or a `static` (their bytes hold addresses), given to a generic function (`F: Fn` with a pointer); a
  pointer to an `unsafe` or `extern` function, or to a closure that reads itself; a call through a pointer of a
  type the program makes no pointer of. Recursion through pointers is refused as any recursion is.
- **Cost.** None to programs without pointers: the compiler's tests, the examples and the support matrix are
  unchanged but for the rows that now compile (fnptr_array is 56 cells; a call through a pointer of two functions
  is the test and each call's own arguments).
- **Evidence.** Support matrix: 7 rows better (call_fn_pointer, call_fn_pointer_param, closure_no_capture_fn_ptr,
  fn_pointer, fnptr_array, fnptr_choose, fnptr_param, at every setting), 0 worse. Probes, each matching native Rust
  at the four settings: a pointer returned from a function, a pointer reassigned in a loop, `[f; 3]` then
  elements set, a pointer through `&fn(..)` and an array through `&[fn(..); 2]`, `fn(&mut i16)`, `P::get as fn(&P)
  -> i16`, pointers of two types in one loop, a pointer to another crate's function and one passed into it.

## 2026-11-06 · An i16's saturating `+`, `-` and `*` are the operation on a card, and panic past one

- **Decision.** `saturating_add`, `saturating_sub` and `saturating_mul` on an i16 (the method, or the intrinsic
  rustc calls at both opt-levels for the first two) are the overflow-checked operation, whose flag on CARDIAC
  says the value is past a card, and a panic there in every profile: `the value doesn't fit on a card`.
  `saturating_mul` stays a call at opt-level 2 (`WRITTEN_AT_EVERY_LEVEL`), where rustc inlines a choice between
  `i16::MIN` and `i16::MAX` as values.
- **Why.** Rust saturates at 32767 and -32768, which no card holds. Of two values on a card, a sum or a difference
  is at most 1998 from 0, so Rust's is never saturated, and wherever it is on a card CARDIAC's is the same number;
  a product Rust saturates (999 * 999) is past a card too. So the answer is Rust's wherever a card holds it, and a
  panic, never another number, where it doesn't: as for `abs_diff`, `!` and `<<`, whose values past a card panic
  where Rust's never do.
- **Considered.** Saturating at the card's own limits, 999 and -999: a wrong answer, as Rust's 600 + 600 is
  1200. Refusing them, as nothing past a card can match: the rows are on a card, and the past-a-card rule is
  the one every i16 operation follows.
- **Cost.** A panic where Rust gives a number past a card (1200, or 32767 for 999 * 999), which Rust never does.
  The three matrix rows match at every setting (30, 30 and 52 cells).

## 2026-11-06 · `isqrt`, `pow` by a variable, `checked_pow` and a u8's `saturating_mul`, written here

- **Decision.** `methods` writes these where rustc calls them, and the driver keeps `isqrt` and a u8's
  `saturating_mul` calls at opt-level 2 too (`WRITTEN_AT_EVERY_LEVEL`): what rustc inlines is core's root of a
  u16, and a u8 product with its overflow flag. `i16::isqrt` panics below 0, as Rust's does, and is then the odd
  numbers 1, 3, 5, .. taken away while they last (their sums are the squares): at most 31 rounds, all on a card,
  no multiply. `i16::pow` by a variable is that many checked multiplications, counted down; its u32 exponent is a
  `wide` cell, which may hold a number equal to it modulo 2^32, and one below 0 (`n as u32` of an n below 0) is
  an exponent past a card, which panics. `i16::checked_pow` by a constant up to 10 (`enum_methods::checked_pows`)
  is `None` where Rust's is, where the power is past an i16, which the base's bounds say without the power (above
  31 or below -32 for a cube), and else `Some` of the power, its multiplications checked against the card in
  every profile. A u8's `saturating_mul` is a sum of b a's that stops past 255, so nothing past 510 is ever held.
- **Why.** Each is exact wherever Rust's answer is on a card. `checked_pow` gives Rust's `None` where it is
  `None`, which a check of the card alone would not: `10.checked_pow(3)` is `Some(1000)` in Rust, a value no card
  holds, so it panics rather than say `None`. A u8's product reaches 65025, which no card holds and the multiply
  routine refuses, so `saturating_mul` doesn't multiply.
- **Cost.** A panic where Rust's value is past a card (`10.pow(3)`, and `2.pow(16)` without overflow checks,
  which Rust wraps to 0), and for `pow` with an exponent past a card even where Rust's answer is on one
  (`1.pow(-3i16 as u32)`). `checked_pow` by a variable stays refused, and by 7 or more doesn't fit in the cells.
  `saturating_mul` of a u8 takes up to 255 rounds. The four matrix rows match at every setting (isqrt 35 cells,
  pow by a variable 78, checked_pow 79, saturating_mul 69); nothing else changed.

## 2026-11-06 · An isize is an i16

- **Decision.** After the C-like enum pass, every isize (a type, a constant, a path's `isize`) is an i16, as an
  i32 is (`front::IsizeIsI16`). The enum pass already made a discriminant's isize one, but only in a program with
  a C-like enum.
- **Why.** On a card an isize and an i16 hold the same values and their arithmetic, comparisons and casts agree:
  `-1isize as usize` is usize::MAX, held below 0 as `-1i16 as usize` is, and back. Its methods are an i16's there
  too, and `isize::MAX`, which no card holds, is refused or settled as `i16::MAX` is.
- **Cost.** Past a card it panics or keeps three digits as an i16 does, where Rust's isize goes on to 2^63. The
  matrix's int_isize matches at every setting; nothing else changed.

## 2026-11-06 · A u16 that is compared is a cell that holds its value, never below 0

- **Decision.** `wide::u16_cells`, before `wide_locals`: the program's own u16 locals (not a parameter or the
  result, and not a local of a function rustc inlined) are i16 cells that hold the u16's value, where every line
  that names one is a cast to or from it, a copy, `+`, `-`, `*`, `/`, `%` (overflow-checked or not), a comparison,
  `&`, `<<` or `>>` by a constant, a switch whose arms are on a card, or an assert's message. The value is never
  below 0: a cast from an i8, an i16, an i32 or an isize is checked to be 0 or more and a cast from a usize to be
  at most 999 (a usize past a card is held below 0), and a difference is checked not to go below 0, in every
  profile. With overflow checks on that check is Rust's own (`attempt to subtract with overflow`); without them
  Rust's value is at least 65536 - 999, and it panics as a value past a card does (`the value doesn't fit on a
  card`). A group of locals that are named together is made cells whole or not at all, and is left to
  `wide_locals` where that makes every one of them a cell.
- **Why.** `wide_locals` keeps a u16 cast from an i16 as a number equal to it modulo 2^16, which is exact for what
  core's `abs_diff` does with one, and compares with a constant as a usize; but two such cells can't be compared
  with each other (-3 held for 65533 and 5), nor divided. A u16 that is has to hold its value, and a u16's value
  is never below 0.
- **Cost.** Where Rust's u16 is past a card, CARDIAC panics or (a sum or product without overflow checks) keeps
  three digits, as for an i16: `(a - b) as i16` of 300 and 400 is -100 in Rust without overflow checks, and a
  panic here, where the modular cells would have been exact had nothing compared them. The matrix's int_u16
  matches at every setting; nothing else changed.

## 2026-11-06 · A formatted panic message keeps its literal text, without the values

- **Decision.** `panic!("negative card: {}", a)`, and `assert!`, `unreachable!` and `assert_eq!` with a message
  that has arguments, become the call a bare `assert!` makes, as a literal message does, and the map gives the
  message's literal text with the values left out (`negative card: `). At opt-level 1 and 2 rustc builds the
  message in the panic's block: its template a byte string, which `panic_messages::template_text` reads in core's
  encoding (pieces, each its length and its bytes; placeholders, each a byte with the top two bits set and the
  fields its low bits say; a 0 byte at the end), and its arguments an array of `fmt::rt::Argument`s. It is
  refused unless the template reads to its end exactly, every placeholder names an argument there, no width or
  precision comes from an argument, every argument points to an integer, a bool, a char or a str (or a reference
  to one), and some text is left that isn't space (`panic!("{}", a)` has none).
- **Why.** Three rows of the support matrix were refused only for the message. The values can't be in the map,
  which is written before the program runs (as for `assert_eq!`'s), but the text says which panic it was.
- **Considered.** A value of the program's own type is formatted by its own `Display` or `Debug`, which could
  do what the program can see (call `out()`), and is dropped with the message: refused. A message built by calls
  (`Arguments::new` and `Argument::new_display` left as calls, at opt-level 0) is refused as before.
- **Cost.** None: the panic's block keeps no statements, as for a literal message. The three rows match at every
  setting; no test program or example changed.

## 2026-11-06 · The bit methods are the intrinsics rustc leaves, written as walks over the bits

- **Decision.** `count_ones`, `leading_zeros`, `trailing_zeros`, which rustc leaves as calls of the intrinsics
  `ctpop`, `ctlz` (`ctlz_nonzero`) and `cttz` on a u8, a u16 (an i16's) or a usize, and `rotate_left` and
  `rotate_right` of a u8 by a constant, are code of the caller's own (`bits`): the low ten bits of the cell walked
  from the top as `runtime::bitwise` walks them, until all that is left is 0. A cell below 0 stands for a value
  whose high bits are 1, a u16's (`wide`) or a usize's past a card: its ones are the bits less those of -1 - c, its
  leading zeros none, its trailing zeros those of -c. The u32 they return is a cell, as `wide` makes one of a count
  known to be on a card. A u8's rotation by one is 2v, less 255 past 255, done k times. `T::MAX >> n` by a variable,
  which `next_power_of_two` makes, is 0 doubled and 1 added (bits - n) times, checked for a usize, whose
  `usize::MAX >> 0` is usize::MAX. `is_power_of_two` is rustc's own code, `count_ones() == 1`. `wide` lets a shift by
  a u32 and a bit intrinsic read a value held modulo its size, which has the bits they read.
- **Why.** The counts are small and on a card; only the bits behind them aren't decimal, and the walk needs no
  more than adds.
- **Cost.** About 20 to 40 cells a call (the walk is written where it is called), and up to ten passes. Left refused: a
  usize's `leading_zeros` (rustc casts it to u64), rotations of an i16, by a variable, and `swap_bytes`, which on an
  i16 is past a card for most values (256 becomes 1, but 100 becomes 25600).
- **Evidence.** Every value on a card for the i16 methods, every u8 for the u8 ones (rotations by 0, 1, 3, 5, 7
  and 9), every value on a card cast to a usize, those below 0 too, against native Rust at all four settings. The
  support matrix's rows for them match at every setting but `swap_bytes_i16`.

## 2026-11-06 · Without overflow checks, the checks of core's inlined code are not made

- **Decision.** In a build without overflow checks, the driver makes an `assert` of an overflow of `+`, `-`, `*`, a
  negation or a shift go straight on to its target (`front::drop_inherited_overflow_checks`), and the checked
  operation it read the flag of, where nothing else reads it, is the plain one. rustc writes no such check in such a
  build, but keeps those of core's code it inlines that inherits the caller's overflow checks
  (`#[rustc_inherit_overflow_checks]`), and codegen leaves them out (`is_optional_overflow_check`). A divide's
  checks stay.
- **Why.** `u8::next_power_of_two(200)` is 0 natively without overflow checks, and panicked on CARDIAC at its
  `+ 1`.
- **Cost.** None found; an i16 past a card keeps three digits there, as `a + b` does in the program's own code
  without overflow checks.

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

## 2026-11-06 · cargo-cardiac says when a newer version is out

- **Decision.** After `build`, `run` and `view`, cargo-cardiac says in one line when crates.io has a newer version
  than itself (`note: cargo-cardiac 0.1.4 is out (you have 0.1.3): cargo install cargo-cardiac`, or `brew upgrade
  cargo-cardiac` for Homebrew's copy), as rustup and gh do (`src/update.rs`). At most once a day it fetches its
  entry in crates.io's sparse index with curl, in the background, into `.cardiac/cargo-cardiac.index`, and what it
  says comes from the last answer. `CARGO_CARDIAC_NO_UPDATE_CHECK=1` turns it off, and it is off with `-q`,
  `--message-format=json` and when standard error isn't a terminal.
- **Why.** `cargo install` never looks again at what it installed, so nothing told a user that a new version was
  out unless they used Homebrew or watched the releases.
- **Considered.** Checking before the build and waiting for the answer: a build would wait on the network once a
  day, and offline for curl's timeout. A crate for HTTP: the driver's download already uses curl.
- **Cost.** One small request a day, of the public index only; the first build after a release says nothing, and
  the one after it does.

## 2026-11-06 · A generic struct's instance is a struct of its own

- **Decision.** Before the struct pass, each instance of one of the program's generic structs (`Pair<i16>`, built
  as `Pair::<i16> { .. }`) is renamed a plain struct of its own, `Pair__i16`, as a tuple of scalars is
  `tuple__i16_i16` (`structs::instances_as_structs`). Another crate's generic struct is left to the passes that
  know it: core's `Skip<Range<i16>>` prints as one word too, so the driver says which structs are the program's.
- **Why.** The struct pass knows a struct by its name alone, so it left every path with type arguments alone, and
  a `Pair<i16>` local was refused wherever rustc didn't take it apart first: where its method stays a call
  (`#[inline(never)]`, or an instance of the entry below). By name, `Pair<i16>` and `Pair<bool>` would have shared
  one layout; as `Pair__i16` and `Pair__bool` each has its own cells.
- **Cost.** None found: a name per instance, which only the listing shows.
- **Evidence.** The support matrix's two new rows, impl_generic_call and impl_generic_two, match at every setting;
  the rest of the matrix, the compiler's tests and the examples are unchanged.

## 2026-11-06 · A generic function's instance is a function of its own

- **Decision.** Where rustc leaves a call to an instance of one of the program's generic functions
  (`bigger::<i16>`, `show::<Sq>`, `sum::<3>`, a trait's default method `<A as Shown>::show`), the driver makes the
  instance a function (`driver/src/instances.rs`): the generic MIR with the instance's arguments put in, as codegen
  does, named by the instance in plain words (`bigger__i16`, `A_as_Shown__show`, a number after it if that is
  taken), and the calls to it call that name. Instances are followed through the calls in their bodies (up to 64).
  What a generic body does through a trait on a type now known is then what rustc writes for that type: a `drop`
  of what needs none is no drop; `PartialOrd`'s and `PartialEq`'s comparisons, `clone`, and `+`, `-`, `*` of two
  integers of one type (comparisons of bools too) are the operation, the arithmetic checked as the program's
  overflow checks say (core's are `#[rustc_inherit_overflow_checks]`); a comparison of a C-like enum whose
  `PartialOrd` (or `PartialEq`) is derived compares the discriminants, as the derived `partial_cmp` does; a
  constant named through the parameters (`T::MAX`, an integer or a bool) is evaluated; an unwind to a cleanup
  block continues. core's own `Ord::max` and `Ord::min` on a C-like enum (`if other < self { self } else { other
  }`) are made the same way, which `x.max(y)` on a derived-`Ord` enum calls at every opt-level.
- **Why.** A generic function worked only where rustc inlined it, and `#[inline(never)]`, a trait reached through
  `impl Trait`, or a call too costly for the inliner (the landing pads of the generic body count against it) left
  it refused at every setting. An instance is an ordinary function once its types are known, and every pass after
  the driver takes it as one, with its cells and its calls checked as any function's.
- **Considered.** Forcing rustc's inliner to inline the program's generic functions: it gives optimized MIR, but
  breaks `#[inline(never)]` and copies the body into every call. Running rustc's MIR optimizations on each
  instance: they are private to `rustc_mir_transform`.
- **Cost.** An instance is the generic body as rustc optimized it before its types were known, which the passes
  after clean up as they can: fn_generic_not_inlined (`bigger::<i16>` and `bigger::<bool>`) is 63 cells, as the same
  program with an `i16` helper and a `bool` helper written out is. Each instance is a copy, as Rust's are. Left
  refused: a generic function of another crate (the linker says so), an instance with a promoted constant, a closure
  as an argument, a const argument the IR doesn't read (`f::<-3>`), and `max` on a struct, which passes it whole. (A
  generic struct whose method stays a call was refused too, until the entry above.)
- **Evidence.** Support matrix against tonight's merged results: 10 rows better, 0 worse; no listing or example
  changes.

## 2026-11-06 · `i16::ilog10` stays a call at every opt-level, and the compiler writes it

- **Decision.** The driver keeps `i16::ilog10` a call at opt-level 2 and 3 too (`inline::provide_written`,
  `WRITTEN_AT_EVERY_LEVEL`), as it keeps the methods the compiler writes at 1 and s; `methods` writes it as it
  does there: a panic below 1, then tests against 10 and 100.
- **Why.** What rustc inlines is core's `int_log10::less_than_5`: four additions of u32 constants (393206,
  524188, 916504, 514288), two ands, an xor and a shift, a trick on the bits of a number below 100000. No card
  holds the constants, and reading the trick back as compares would be pattern matching on core's code for a
  method the compiler already writes.
- **Cost.** None found: the row is 33 cells at every setting, as at opt-level 1. `checked_ilog10` and a u8's
  `ilog10` stay refused (an `Option<u32>`, a `NonZero<u8>`).

## 2026-11-06 · `div_euclid` is one divide: `(a + 1) / b` less b's sign for a below 0

- **Decision.** `methods`: where rustc calls `i16::div_euclid`, it is `a / b` for a not below 0, and for a below
  0 `(a + 1) / b`, one less for b above 0 and one more for b below. That is Rust's answer for every a below 0, b
  dividing it or not (`(-7).div_euclid(7)` is -1, and so is -6 / 7 - 1; `(-8).div_euclid(7)` is -2, and so is
  -7 / 7 - 1), and a + 1 of a number below 0 is on a card. It replaces the quotient and the remainder, the remainder's sign telling the fix.
- **Why.** The divide routine once, not the divide and remainder routines both: the matrix's row by a variable is
  88 cells, where it didn't fit in 96 (a `/` by a variable alone is 66), and by a constant 68 where it was 81.
- **Cost.** Two tests of a's sign where there was one of the remainder's.

## 2026-11-06 · A core method's i32 compiles where its values are on a card

- **Decision.** `front::widened_in_core` refuses an integer method of core's, of a type other than i32, that rustc
  inlined with i32 locals, only where those locals can hold a value past a card. What each of core's i32 locals
  here can hold is worked out from its assignments, round after round until nothing grows: a constant, a cast from
  a u8 (0 to 255), an i8 (-128 to 127) or an i16 (a card), and `+`, `-`, `*` and negation of those. Any other assignment, a write through a reference or a projection, or a call's result, may
  hold anything. `u8::abs_diff` at opt-level 2 (the two u8s' difference in i32, then its absolute value: -255 to
  255) compiles; `i16::midpoint` (the two i16s' sum in i32: -1998 to 1998) stays refused.
- **Why.** Core works in i32 to reach past the narrower type, and a card is narrower than i32; but where every
  value is on a card, i32's arithmetic and a card's agree, overflow checks or not. Of core's integer methods on
  i8, u8, i16, u16, isize, usize and u32 (about 400 tried), only these two have i32 locals at opt-level 1 or 2.
- **Cost.** A method whose i32 values stay on a card for reasons the spans can't see (`midpoint` of two small
  numbers) is still refused.

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

## 2026-11-06 · `swap` and `reverse` at opt-level 2: one-element copies, slices from parts, pointers to locals

- **Decision.** What rustc inlines of a slice's `swap` and `reverse` at opt-level 2 is read: `copy_nonoverlapping`
  of one element, and `core::intrinsics::copy` of one, are the assignment `(*dst) = copy (*src)`;
  `_t = const <uninit>` is no statement; `*mut [T] from (p, n)` with n a constant is p cast to a pointer to `[T;
  n]` (the length in the type, where `slices` finds it); `MaybeUninit<T>` of a card's T is T; and `revswap(a, b,
  n)`, the swaps `reverse` calls, is written out (n up to 8). After the struct pass, a pointer assigned once to
  the address of a whole local (or a copy of one), used only as `*p`, is that local (`pointers_to_locals`), and a
  pointer into an array through such a pointer to the whole array is an index into the array.
- **Why.** `swap` moves one element through a temporary; with these it is the four copies it is.
- **Cost.** None found. A `copy_nonoverlapping` of more than one element, and a slice from parts of a length not
  known, stay refused.

## 2026-11-06 · A slice iterator's and a StepBy's calls at opt-level 1, as what rustc inlines at 2

- **Decision.** `slices::slice_iterator_calls`: where rustc calls them, `iter()`, `iter_mut()` and `into_iter()` of
  an array reference build the slice iterator's two pointers, `into_iter()` of the iterator (or of an `Enumerate`
  of it) is the iterator, `next()` is its test and step, and `enumerate()` builds core's struct: the code rustc
  inlines at opt-level 2, which the passes after take as they do there. `iterators::step_by` likewise reads a
  `StepBy`'s `into_iter()` and `next()` (the range's `nth()` of 0 the first time, of k - 1 after).
- **Why.** One way through the passes for both opt-levels: the loops take the same cells at 1 as at 2.
- **Cost.** None found.

## 2026-11-06 · rust-cardiac is the project, cargo-cardiac the command, `cardiac` the library; cardiaq is gone

- **Decision.** Packaged as pgrx (`pgrx` and `cargo-pgrx`) and rust-psp (`psp` and `cargo-psp`) are. The project
  and its repository are rust-cardiac. What you install is cargo-cardiac, which now does everything from the first
  line to the last run: `cargo cardiac new` (`cargo new`, then `cardiac` added and a countdown as `src/main.rs`),
  `setup` (the driver and its nightly fetched up front, each said with its size), `build`, `run` (build, then the
  deck run on burn with the panic map beside it, each phase's deck as burn loads them), `debug` (ekg) and `asm`
  (carc). What a program uses is the `cardiac` crate, which `new` writes into its Cargo.toml. The engine is
  heartburn: cargo-cardiac depends on it with its `tools` feature, which gives `heartburn::tools::{burn, carc,
  ekg}::main`, so run, debug and asm are those programs under cargo's names. cardiaq, the crate of the tools and
  the name of the runtime, is retired, and nothing tells anyone to install it. burn, carc and ekg on their own,
  for people without Rust, come from `brew install heartburn` and heartburn's release archives. The repository's
  README, headed rust-cardiac, is also the `cardiac` crate's; cargo-cardiac has a short one of its own,
  CARGO-CARDIAC.md, that lists the subcommands. All three crates are 0.1.2.
- **Why.** "Install cargo-cardiac, and cardiaq for burn" was two installs, two names and a third for the library
  before a first program ran. One install and four commands (`cargo install cargo-cardiac`, `cargo cardiac new
  countdown`, `cd countdown`, `cargo cardiac run`) is the quick start, as with pgrx and rust-psp.
- **Cost.** cargo-cardiac carries burn, carc and ekg's code (heartburn with `tools`), about what cardiaq's
  install was. The tests still run decks on a burn of their own (`BURN`, or burn on the PATH), so a contributor
  still needs one. cargo-cardiac can't be published until heartburn 0.1.2 is on crates.io.

## 2026-11-06 · A wrapping `*` is refused where the product can pass the accumulator

Superseded the same day by "A wrapping `*` past a card is what CARDIAC keeps of it", above.

- **Decision.** A `u8`'s or an `i8`'s wrapping `*` (with overflow checks off, or `wrapping_mul` inlined) that the
  range pass hasn't shown stays in its type's range is the plain multiply and then the wrap, only when one side is
  a constant from -39 to 39, so the product is at most 9999, what the accumulator holds. Any other is refused.
- **Why.** Two `u8`s' product reaches 65025, and the multiply kept what was left of it: `200u8.wrapping_mul(200)`
  was 0, not 64 (cargo-cardiac-support#142).

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

## 2026-11-05 · A byte string is an array of its bytes, built where it is used

- **Decision.** The IR reads a byte string constant (`b"CARD"`, `Const::Bytes`, escapes as rustc writes them). The
  front end makes each use a `[u8; N]` of the bytes, in a local of the function's own at the start of the block,
  and its borrow, as a promoted constant is built.
- **Cost.** N cells a use, and in a function other than main's opening blocks N stores.

## 2026-11-05 · A static is a fixed run of cells, as every variable on CARDIAC is

- **Decision.** The IR reads a static (`Item::Static`), the allocation rustc says is its (`Item::Alloc`,
  `alloc1 (static: T, ..)`), and a pointer into an allocation (`Const::Alloc`, `{alloc1: &[i16; 4]}`). A static
  whose first value is a literal of numbers or bools (or an array of them) is cells in the data section, with
  those values, `static_NAME`; a pointer to it is the constant holding its address, so every read and write goes
  through it, from any function, as through any raw pointer. `static mut` is the same cells: a call that changes
  one is seen by its caller, since no pass keeps what a pointer read gave across anything.
- **Why.** Every CARDIAC variable is a fixed cell already; a static is one no function owns.
- **Cost.** Each access is through the address (self-modifying code), a few cells and steps more than a local's.
  A dependency's statics have no MIR in its metadata, and stay refused; so does one rustc works out otherwise.
- **The oracle.** rustc's text writes a pointer to a static `&[u8; N]` as `<static(DefId(..))>`, which it can't
  read: the driver gives it as the allocation, as for any other type, and the two differ there.

## 2026-11-05 · A const table rustc works out is the literal it works out to

- **Decision.** For a const array of numbers or bools whose body works it out (`[A, B, A + B]`, a const fn's
  table), the driver gives the body as the literal rustc evaluates it to (`const_eval_poly`), which the front end
  reads as any const table. Tables of `i8`, `usize` and `bool` are read as those of `i16` and `u8` are.
- **The oracle.** The text keeps the body that works the value out, so such an item differs from the text's
  there; no test program has one.

## 2026-11-05 · A call of a function to itself, as the last thing it does, is a loop

- **Decision.** In the front end, `f`'s call of `f` whose result is `f`'s own and after which nothing but the way
  to the return comes is each argument into a temporary, each temporary into its parameter, and a jump to the
  function's start (a block of its own that the entry goes to). The parameters' cells hold the next round's values,
  as the callee's would; nothing else of the caller is read after the call. Not where a parameter or an argument is
  a reference or a pointer, which could point into the cells reused. Other recursion still needs a stack.

## 2026-11-05 · An integer method of core's that works in i32 is refused

- **Decision.** The IR keeps, for each body, the locals rustc declared in a function it inlined and that
  function's path, as the scope says it (`scope 3 (inlined core::num::<impl i16>::midpoint) {`; `Body::inlined`,
  read from the text and from rustc's source scopes alike). A function into which rustc inlined an integer method of
  core's, of a type other than i32, that has an i32 local (`i16::midpoint`, which adds the two in i32) is refused:
  "core::num::<impl i16>::midpoint, inlined in main, works in i32 so that it can't overflow, and on CARDIAC an i32
  is a card".
- **Why.** i32 is i16 on CARDIAC (`front::I32IsI16`), which is right for a program's own i32: on a card either way,
  and past one it differs as an i16 does. But core widens to i32 only so as to work past what the narrower type
  holds, and on a card that is a wrong answer for values the program never sees: `999.midpoint(998)` printed 598.
  An i32 method's own i32 (a range of i32's step, `<impl i32>::overflowing_add_unsigned`) is the program's, and
  stays.
- **Cost.** `i16::midpoint` doesn't compile at any opt-level (at 1 and s it was already a refused call).

## 2026-11-05 · No card holds i16::MIN or i16::MAX: a test against one is settled, a value refused

- **Decision.** Before any pass, at every opt-level (`settled_by_the_card`), a comparison of an i16 with
  `i16::MIN` or `i16::MAX` is the constant the card makes it (`x == i16::MIN` false, `x < i16::MAX` true), with an
  `&` or `|` of it that one side settles and an assert of the result; and a switch on an i16 loses its arms past a
  card (`match x { i16::MIN => .. }`). Anything else that reads a limit is the value, which is refused as any
  constant past a card is: "constant out of range: i16::MAX (32767) is past what a card holds". The assembler
  refuses a `DAT` past a card too, should one get that far, rather than writing a deck burn can't load.
- **Why.** A limit used as a value (`i16::MAX - a`, or what `saturating_mul` saturates to) became a cell of 32767,
  and the compiler crashed running it (heartburn's "a card holds -999..999"); so did a test against one where
  lowering's own special case (`==` and `!=` with the limit on the right) didn't reach it. rustc writes these tests
  in its own checks (`-x`, `x / y`) and in the methods it inlines at opt-level 2.
- **Cost.** None measured: no test program or example changed size. `checked_neg`, `checked_div` and
  `checked_rem` on an i16 now compile at opt-level 2 (the test for i16::MIN goes, the test for 0 stays), and
  `saturating_mul` is refused where it crashed.

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

## 2026-11-05 · `Ordering` is a C-like enum; `Cmp` is two tests

- **Decision.** rustc's three-way compare is an operation of the IR, `BinOp::Cmp` (the driver used to hand it on
  as a variant named `Cmp`). The front end makes each `d = Cmp(a, b)` two tests before the C-like pass, `a < b`
  then `a > b`, setting d to `Ordering::Less`, `Greater` or `Equal`, which the C-like pass then numbers from the
  driver's facts (-1, 1, 0); the block is split at the `Cmp`. At opt-level 1, `<T as Ord>::cmp(&a, &b)` on an
  integer is that `Cmp` of what the references borrow, `Ordering::is_lt` and its kin are a comparison with
  `Equal`, `then` is a test of the first for `Equal`, and `reverse` is `Cmp(Equal, o)`. A local holding a C-like
  enum's discriminant in a type the backend lacks (`Ordering` is `#[repr(i8)]`) is an i16, with the constants it
  is compared with, but only where it is set by `discriminant` alone and only compared, switched on or cast:
  those give the same answer in both types, where arithmetic in an i8 could overflow where an i16 doesn't. A
  variant rustc names alone (`const Less`, in core's code it inlines) is the one of the enum it is set to or
  compared with.
- **Why.** `a.cmp(&b)`, `match` on an `Ordering`, `is_lt`, `then`, `partial_cmp` and `signum` at opt-level 2,
  and the derived `Ord` and `PartialOrd`, all were refused for want of `Cmp`, and an `Ordering` is a cell already.
  Two tests, not a subtraction and its sign: a - b may be past a card (`999.cmp(&-999)`).
- **Cost.** Two tests and three stores of a constant: the matrix's `out(a.cmp(&b) as i16)`, three times round, is
  27 cells at both opt-levels, as is the `match` on it, where jump threading takes each constant straight to its
  arm. No test program or example changed. Still refused: `partial_cmp` at opt-level 1 (it builds an
  `Option<Ordering>` in a way the enum pass refuses), and the derived `Ord` and `PartialOrd` methods rustc calls
  at opt-level 1 for a C-like enum (`<Lvl as Ord>::max`, `<Lvl as PartialOrd>::lt`), whose bodies are not compiled.

## 2026-11-05 · `i16 as u8` wraps as Rust's does; a usize compared with a constant past a card

- **Decision.** `x as u8` of an i16 is x mod 256, as Rust's two's complement gives it: a negative x is raised by
  256s, then the value lowered by 256s until it is below 0 and raised once (the code `usize as u8` already had for
  a usize past a card). Where the range pass knows x is 0..=255 the cast is a copy. The cast now takes the type of
  what it casts from the place, so an array's element or what a reference points to is cast by its type: before,
  `t[i] as u8` of an i16 element was stored as it was (-1 stayed -1, #139). A usize compared with a constant k
  past a card (1000 up to 2^64 - 1000), as rustc's inlined `i16::try_from(n)` tests `n > 32767`, is compared with
  999: a usize on a card is at most 999, and one past a card, held below 0, is above every k.
- **Why.** `i16 as u8` was refused, and so `u8::try_from` of an i16 at opt-level 2 (the range check, then the
  cast), and every row that has to read its u8s from a table; `i16::try_from` of a usize at opt-level 2 was refused
  for its constant.
- **Cost.** A cast the range pass can't settle is about 12 instructions and a pass of a three-instruction loop
  per 256 (at most four). `TryFrom` at opt-level 1, where rustc calls `try_from`, is still refused: the call
  builds a `Result` whose `Err` holds a `TryFromIntError`, a struct the enum pass has no cells for.

## 2026-11-05 · u16 and u32 locals as cells, where the program can't tell; more integer methods

- **Decision.** A u16 or u32 local (not a parameter or a result) is an i16 cell in one of two ways (`wide`). Where
  its value is known to be on a card, 0..=999 (a constant, a cast of a u8 or a bool, a sum, difference or quotient
  by a constant of such values whose bounds stay on the card, or the result of `abs_diff`, `unsigned_abs` or
  `ilog10` on an i16), the cell holds the value and the local is used as any number is. Otherwise, where it is a
  cast of an i16, an i32 or a usize, or a sum or difference of such, the cell holds a number equal to it modulo
  2^16 (2^32): `-500i16 as u16`, 65036, is -500. Such a local may only be cast to i16 or u8, or added to or
  subtracted from into another, which keep the equality; each of those sums and differences is overflow-checked
  in every profile, so a cell never holds a wrong number. Anything else (a comparison, a division, a wider cast, a
  call) and the local keeps its type, which the lowering refuses. The methods, where rustc calls them: on an i16,
  `abs_diff` (the subtraction checked, as 999 and -999 are 1998 apart), `unsigned_abs` (`abs`), `ilog10` (a panic
  below 1, then tests against 10 and 100), `wrapping_div` and `overflowing_div` (`/` and its check of a zero
  divisor; no card holds i16::MIN), `div_euclid` by a variable, and `pow` by a constant up to 10 (its
  multiplications checked in every profile); on a u8, `saturating_add` (also the intrinsic, at opt-level 2) and
  `midpoint`, whose sum an i16 cell holds.
- **Why.** These methods' answers fit on a card; what was refused was the wider type rustc works in, or the call.
  A wider type can hold what a card can't (`-500i16 as u16` is 65036, and core's `abs_diff` relies on it wrapping
  back), so it is only a cell where the program can't tell the difference, and only with the checks that keep
  the cell's number right.
- **Cost.** Where Rust's answer is past a card, CARDIAC panics in every profile, though Rust wouldn't (`(-999).
  abs_diff(999)` is 1998; `10.pow(3)` with overflow checks off). A `pow` costs the multiply routine. Still refused:
  `ilog10` at opt-level 2 (rustc inlines bit operations on constants past a card), and a u16 compared or divided
  where it may have come from a negative i16. `div_euclid` by a variable compiles but costs the divide routine
  and the remainder routine both: the matrix's row doesn't fit in 96 cells.

## 2026-11-05 · `!`, `<<` and `>>` by a constant on an i16 and a u8, and `^` of two bools, as arithmetic

- **Decision.** `methods::inline_operators` writes each as the arithmetic it is in two's complement, before the
  passes, so the range pass and the rest see ops they know. On a u8: `!a` is `255 - a`; `a << k` is the low
  8 - k bits of a times 2^k (`(a & (2^(8-k) - 1)) * 2^k`, never past 255, so nothing wraps); `a >> k` is the
  quotient by 2^k. On an i16: `a >> k` is `a.div_euclid(2^k)`, worked as the quotient of a for a not below 0 and
  `-1 - (-1 - a) / 2^k` below it, so only values not below 0 are divided (one short unsigned loop, not the signed
  divide and remainder routines: 46 cells for the row, where `div_euclid`'s own code took 81). `a ^ b` of two bools
  is `a != b`. k goes up to 9 on an i16 (512 is the last power of two a card holds) and 7 on a u8; past that, by a
  variable, and on a usize they stay refused, now with a message that says so.
- **Where Rust's value is past a card.** Rust's `!` and `<<` never panic over the value (overflow checks look at
  the shift amount only), but on an i16 they can make a value no card holds: `!999` is -1000, and `a << k` is
  a * 2^k wrapped in 16 bits (`300 << 7` is -27136; `513 << 7` wraps all the way round to 128, which a card does
  hold). A card can't keep 16 bits, so the result is checked against a card with overflow checks on and off
  alike, and the program panics there (`the value doesn't fit on a card`), rather than keep the low three digits
  as an unchecked `+` does: `!a` is `assert(a < 999)` and then `-1 - a`, and `a << k` is `MulWithOverflow(a,
  2^k)` and its assert. So `a << k` panics wherever a * 2^k is past a card, even the few times the 16-bit wrap
  lands back on one: a panic, never a wrong answer. The range pass drops either check where it can prove the
  value fits (`!i` and `i << 3` of a loop counter 0..12 have none). `a >> k` is always on a card.
- **Measured.** Every value a card holds, through every k, on both types, at opt-level 1 and 2 with overflow
  checks on and off, compared with Rust's value: no wrong answer, and a panic only where Rust's value (for `<<`,
  a * 2^k) is past a card. Five matrix rows refused before match at every setting; no test program or example
  changed.

## 2026-11-05 · Literal panic messages go in the panic map; `assert_eq!` says Rust's first line

- **Decision.** A panic whose message is a literal (`panic!("negative card")`, `assert!(a > 0, "...")`,
  `unreachable!("...")`, `unimplemented!("...")`, `todo!("...")`) and every `assert_eq!` and `assert_ne!`, with a
  literal message or none, become in the front end the call a bare `assert!` makes, `core::panicking::panic(const
  "message")` (`panic_messages`), and `panic`'s string is the message the map gives the check, as `expect()`'s is.
  rustc builds a `core::fmt::Arguments` for the message: at opt-level 1 with a call to `Arguments::from_str` in
  the block before the panic, which becomes a goto; at opt-level 2 in the panic's block, its template the
  string's address and its `args` the length shifted with the low bit set, which is what marks a template that is
  only a string. The panic's block keeps no statements (nothing it did can be seen once the program has
  panicked), and the locals that only fed the message go, with their declarations: `assert_eq!`'s borrows of its
  two values, the `Arguments`, the string. `assert_eq!(a, b)` calls `core::panicking::assert_failed` with the two
  borrows; its message is Rust's first line, ``assertion `left == right` failed`` (`!=` for `assert_ne!`), and
  `: message` after it for one with a message. A bare `assert!(c)` and `unreachable!()` now say what Rust says
  (`assertion failed: a > 0`, `internal error: entered unreachable code`), where the map said `explicit panic`.
- **Why.** Seven rows of the support matrix were refused only for the message, and the comparison before the
  panic was already compiled (`assert!(a == b)` worked).
- **Cost.** Rust prints the two values under `assert_eq!`'s first line (`  left: 7` and ` right: 3`); the map
  doesn't. It is written before the program runs, and its format has one message a check, so it would need the
  cells the values were in at the panic, which the compiler shares with other locals by then. A message with
  `{}` arguments (`panic!("bad card: {}", a)`) is no literal and stays refused, as do `assert_matches!` and a
  message built any other way. No program grew: `assert_eq!(a, b)` is the same 20 cells as `assert!(a == b)`.

## 2026-11-05 · Listings use the source's names

- **Decision.** A listing (and the assembly) names a cell after the variable the MIR's debug info says it holds
  (`debug height => _9;`): `height` where it said `main_9`, `height_1` for a field of it, `a_height` for the
  constant holding its address. A name two functions use keeps the function in front (`main_n`, `add_n`), a
  shadowed one is numbered (`n`, `n_2`), and one that reads as a name the compiler makes (`k5`, `t0`, `main_bb3`,
  `rt_...`) or an opcode keeps its function too. When a pass makes two variables one local, or rustc gave one
  local two names, the cell takes the name declared first and its data line notes the other (`also colour`).
  Comments say the MIR statement in Rust's terms: `copy` and `move` dropped, constants without their type,
  arithmetic, comparisons and `Lin` written infix (`height = cell / 10`, `if row < 96`); any other shape stays as
  the MIR prints it, its locals named. Unnamed temporaries stay `_12` and `main_12`. The names are worked out
  before lowering (`names::Names::of`), the comments rewritten as each function is lowered, and the cells renamed
  on the finished program before it is assembled, so every cell stays where it was and the deck is the same.
- **Why.** `main_9` and `_9 = Div(copy _8, const 10_i16)` say what rustc made of the program, not what the
  programmer wrote; a listing is read beside the source.
- **Cost.** A shared cell carries one name, so a comment on a line working the other variable reads in the first
  one's name (`height = camera - height` for `let below = camera - height`). A rename that would give two symbols
  one name is not made.

## 2026-11-04 · Decks, not tapes; build is the only command; burn runs the decks

- **Decision.** `cargo cardiac build` writes cardiaq's files: `NAME.cqb` (`NAME.2.cqb`, ... for a program in
  phases) and its slots in `NAME.cqb.meta`, and with `--emit listing` the six-column listing `NAME.cq`, which carc
  assembles. `--emit` takes `deck` where it took `tape`, and `--message-format=json` says `deck`. `cargo cardiac
  run` and `cargo cardiac cardiasm` are gone, and so are the flipboard and the CARDIASM baseline. burn runs a deck,
  and reads the panic map beside it (cardiaq#9), so the map's reading side (`Map::parse`, `Map::panic`, `Panic`)
  moved to heartburn; making a map from a compiled program (`panics::map_of`) stays here. The tests and the fuzzer
  run decks on burn (`BURN`, or burn on the PATH).
- **Why.** Running a deck is the runtime's job, and cardiaq is the runtime: one machine, one set of names, and the
  map read where the deck is run, as a debugger reads a .pdb beside an .exe. Cardigan replaced the flipboard.
- **Cost.** What `run -v` and `-vv` said of a run (each card read and output, where it stopped) is gone; ekg and
  Cardigan show a run in more detail. Building and running are two commands, where `cargo cardiac run` was one.
  A build still removes the `.tape`, `.tape.meta` and `.lst` files an earlier build left.

## 2026-10-24 · `skip_cards(n)` and `add_times(c, n)`: computed jumps into shared rows

- **Decision.** Two built-ins, each a runtime routine put in once a program (as `mul` and `div` are) with a row of
  identical instructions entered by a jump the routine builds from the digit: `rt_skip`, ten `INP 00` (27 cells),
  and `rt_times`, nine `ADD rt_times_c` (34). Each row ends in `TAC 99`: back to the cell after the jump that came
  in, with cell 99 untouched, as long as the accumulator is below 0 there. `skip_cards` enters with a TAC (no JMP,
  so 99 keeps the caller's way back) and counts the tens in the accumulator, -T-1 up to 0, running the whole row
  once more for each: n up to 999, 15 instructions and about 1.2 a card. `add_times` enters with JMPs (it keeps
  its caller's way back in its exit cell): the tens digit from -100, a shift that makes T c - 100 into 10 T c -
  1000, then the units, and 1000 added back. The offsets keep the accumulator below 0 at each `TAC 99` whenever n
  c fits on a card, and n is 0..=99, the tens digit one row's worth: 24 instructions and one an addition. A
  constant n is `INP 00`s in place (n up to 3) or `c * n` (adds and a shift). The cell-sharing pass follows a
  built jump into its row (`rt_x_go...` to the cells before `rt_x_end`), and a label a constant names
  (`DAT rt_times_end+800`) is a use of it. The constants built on a routine's labels are in its code: the data
  section is assembled alone first and can't name them.
- **Why.** A counted loop takes 6 or 7 instructions a pass; programs that read to the nth card or multiply by a
  small count by repeated addition spend most of their time there. The rows are shared, not one per call or per
  function, because 10 cells a call is more than most programs can spare; one per program costs a call 2 or 4
  cells and some 15 to 25 instructions for the routine's own setup.
- **Cost.** About 27 or 34 cells once; for n below about 4 a loop is cheaper. No check: an n out of range, or a
  product past a card, is undefined on CARDIAC (natively both panic). n is capped at 99 for `add_times`: a
  hundreds digit is one more pass, about 7 cells. `skip_cards(-1)` and `add_times(c, 100)` with a constant are
  compile errors.

## 2026-10-22 · `move x` is x's last read, after every pass; a compare's bool is set on the way that reads it

- **Decision.** The lowering trusts `move x` to be x's last read: a comparison moved into a switch or an assert is
  one branch that never stores the bool. Every pass keeps that true. Copy forwarding (`_5 = copy _6;
  switchInt(move _5)`) and cell sharing can rename a move onto a local that is still read later, so after them
  `blocks::copy_unless_last` turns such a move into a copy. And a comparison tested at once and read again later
  (`let e = x > y; if !e { .. } if e { .. }`, or `a && b`) is still one branch: `blocks::bools_on_edges` sets the
  bool as a constant on each way out where it is still read (a block on the edge, or one already setting it), and
  `thread_constants` then takes a constant tested again straight to its arm. Before `thread_constants` it sets b
  only where every way that reads it goes straight to a test of b (threading takes it past); elsewhere it waits
  until after the last pass, since `arith::rematerialize` may yet work b out again where it is read, which costs
  no cell (a larger program's `aim` grew 93 to 97 when b was set early).
- **Why.** Forwarding made `switchInt(move _6)` of a bool tested again later, the branch never stored it, and the
  later test read a stale value (wrong answers at every opt-level).
- **Cost.** A program that was wrong grows by the stores it needs (a larger program's `dhit` as first written,
  91 wrong to 94 right); one whose bool is threaded can shrink (the bug's first reproduction, 38 wrong to 28). Setting the bool on
  the edges is no dearer than storing it as 0 or 1 and testing it: six of that program's machines shrank by 2 to 4 cells
  (dmul 75 to 71), none grew, every render identical.

## 2026-10-22 · `x & m` on an integer is x.rem_euclid(m + 1); other bit operations are refused

- **Decision.** `&` and `|` were lowered as the bool forms (and = 0 if a is 0, else b) whatever the operands'
  type, so `x & 3` on a counter 0..7 was "0 if x < 1, else 3". The bool forms are now for bools only. On an
  integer, `x & m` (the mask on either side) for m one below a power of two is lowered as x.rem_euclid(m + 1),
  which it is in two's complement: subtract m + 1 until the accumulator goes negative and add it back once, and
  for a value that can be negative (an i16, a usize past a card) first add m + 1 until it is not. Any other `&`,
  and every integer `|` and `^`, is refused, as docs/RUST.md already said. The range pass gives `a & c` 0..=c for
  c not below 0 (so `t[x & 3]` on a `[_; 4]` needs no bounds check).
- **Why.** A wrong answer; RUST.md listed `&` on integers as refused, but nothing checked the type. Masks are the
  one bit operation programs use on a decimal machine (an index that wraps, `x & 3`), and the loop is what `%` by
  a variable costs, without the routine.
- **Cost.** About x / (m + 1) passes of a three-instruction loop: 250 for `999 & 3`. Two cells more for an i16.

## 2026-10-21 · The overlay convention: a phase stops with the bug on the next tape

- **Decision.** A phase ends with `HRS n`, not `JMP 00`: the machine stops with the bug (the program counter) on n,
  the number of the tape to load next. The operator puts that tape in the reader, ahead of the input still unread,
  moves the bug to 00 and presses Run; the loader reads it in. The bug on 00 is a stop for good, as every program
  already ends (`HRS 0`, the panic routine's too).
- **Why.** With `JMP 00` the machine never stopped: the loader read the next card at once, so the next tape had to
  be in the reader already. The emulator put it there the moment the jump ran, an operator who knew the future. On
  CARDIAC that works only for a deck stacked beforehand, which needs the number of cards each phase reads known in
  advance; a program like the guessing game, which reads as many cards as the player takes, can't be stacked. A
  stop is something an operator can see and act on, and naming the tape lets the machine ask for any of them
  (loading one again is left for later).
- **Overlay mode belongs to the program.** HRS does what it always did (halt, the bug on its address); only the
  operator's reading of it is new, and only for a program in phases. A program not in phases can still stop on any
  cell to be run on from there ("play again?"). In the emulator: `run` halts at any HRS; `run_phases` (`cargo
  cardiac run`, the tests) feeds the tape asked for; `run_stacked`, for a deck stacked beforehand (as a larger program's is),
  runs on at 00 with the deck's next card. `JMP 00` is a plain jump into the loader again, as on the machine.
- **Rejected: a flag in the accumulator** (a tape number in it at the stop). Every program halts with whatever its
  accumulator holds, a panic with the value that overflowed, so an old program could look like a request; and it
  costs a CLA at each stop.
- **Cost.** None: `HRS n` is one cell, as `JMP 00` was. A larger program's listings changed in 11 lines (`JMP 00` to
  `HRS 02`), its cells not at all, its renders identical to native. The peephole's rules that took any HRS for the
  program's end (a scratch cell or a constant read after it, a load before it) now take only `HRS 0`.

## 2026-10-21 · auto-overlay: the compiler places the phases

- **Decision.** A setting, off by default: `auto-overlay = true` under `[package.metadata.cardiac]` (a single
  file: `--auto-overlay`). On, a program whose main doesn't fit in memory is cut into phases by the compiler, and
  an `overlay()` in main is an error. Off, nothing changes: the program places its phases with `overlay()`.
- **Where it cuts.** Main's optimized blocks, once the passes are done and before anything is decided block by block
  (`phases::cut_points`): on a way from one block to the next, or between two statements of a block, where every run
  passes once on its way to main's return and that is the only way into the rest, so a phase is never loaded twice.
  A main that never returns gets no cut. Each place is compiled and measured; the split kept is the
  smallest in all whose phases each fit with the shared data, a tie going to the smaller largest phase. When no one
  cut is enough, the cut that leaves the largest phase smallest is kept and another is looked for, up to 4 phases.
- **Why a setting, and why overlay() is an error with it on.** The programmer decides whether the compiler
  tries at all, and falls back to `overlay()` with it off; with it on, `overlay()` has no place in the source. A
  program that mixed the two would have its own cuts and the compiler's to reconcile.
- **Phase order.** A phase now starts with the block it is entered at. The loader starts a phase at its first line,
  and blocks came in rustc's numbering; the block a cut returns to can be numbered after others of its phase.
- **Evidence.** A larger program's 11 machines with `overlay()`, the calls taken out and the setting on: eight split with
  exactly the cells of the hand-placed cut (rotor's is mid-block), and solid, compose and floor no longer need a
  second phase (96, 94 and 90 cells); renders identical to native. That program's whole build takes under a second.
  `tests/programs/auto_overlay.rs` (2^60 in digits) runs past memory without the setting and matches native with
  it.
- **Fuzzing.** `CARDIAC_FORCE_CUT=K[,K...]` cuts every program at its Kth places, fit or not; with it, `sh fuzz.sh
  --generate` checks split programs against native in every setting. It found three faults, fixed: cuts inside an
  `if`'s arm (passed by some runs, not all), a cut through a block whose temporary was already a scratch cell, and
  cuts in a main that always panics; and a phase that starts with a panic (below), on main too. Then 1,200 programs, 12 seeds, one to three cuts each,
  about 29,000 runs: no difference.
- **Cost.** A program too big compiles once per place it could be cut, and again per extra phase. Only cuts in
  main, never in a function it calls or inside a loop.

## 2026-10-21 · A phase that starts with a panic keeps its jump to it

- **Decision.** The peephole's look back for "the line before", which takes over a dropped `JMP rt_panic`'s check
  when the panic routine comes next, stops at a phase's start. A phase whose code starts with the panic keeps its
  jump.
- **Why.** The line before was the last of the phase before, a different tape: the check was filed under that
  phase, and `rt_panic`'s label, which nothing named any more, went too, so the panic map had no stop for the phase.
  `cargo cardiac run` punched -999 and exited 0, where Rust exits 101. Found by fuzzing auto-overlay with forced cuts.
- **Cost.** One cell (the JMP), only in a phase that starts by panicking. `tests/programs/panic_phase.rs`.

## 2026-10-21 · A negative word is skipped, and kept for new instructions

- **Decision.** The emulator skips a cell holding a negative number when it is reached as an instruction: a no-op,
  as Brian Stuart's simulator (https://www.cs.drexel.edu/~bls96/museum/cardiac.html) has it, where the 1969 manual
  says nothing. Before, it ran the word's digits (-123 as `CLA 23`).
- **Why.** It leaves the negative words free, an opcode space for new instructions later (-0xx to
  -9xx), where running their digits would have given them a meaning already. The flipboard's two extensions live
  there already, CUT n as -(600 + n) and LDC n as -(100 + n); the emulator, with no card rack, took them for STO
  and CLA, and now passes over them. It also matches the simulator that
  settles the manual's other open questions, and agrees with ours on all of them (a 4-digit accumulator, STO
  keeping the low three digits, cell 00 fixed at 001, JMP writing 800 plus the next address into 99).
- **Cost.** None to compiled programs, which never execute a negative word; only a program that runs into its own
  data behaves differently.

## 2026-10-21 · A later phase's opening constants, dead stores, and ways in that can't be taken

Three small ones a larger program's rewrite in plain Rust found, each a few cells where the plain form cost more:
- **A later phase's opening run**. Main's opening blocks, run once, store their constants as their cells'
  initial values on the tape. So now does the run each `overlay()` returns to, for a local no earlier phase names:
  its cell is punched by a tape before anything reads it. `overlay(); for _ in 0..96 {..}` starts its counter at -96
  on the tape, not with a CLA and STO in phase 2 (phase 2: 10 cells to 7).
- **Dead stores** (`blocks::dead_stores`). A value or a comparison stored where no way on reads it before it
  is set again goes, by liveness: `if (0..36).contains(&c)` left `_2 = const false` on its false way, past the test
  that read it (14 cells to 10, the hand-written `c >= 0 && c < 36`). Only plain locals whose address is never
  taken; a return reads the result.
- **Ways in that can't be taken**. The range pass joins what holds on each way into a block; a way on which
  some range is empty (`233 <= x` on the way out of `x < 233`) is never taken, and is left out of the join. The
  descending `match l { 233.. => 3, 124..233 => 2, 31..124 => 1, ..31 => 0 }` loses its redundant test: 30 cells,
  as ascending, from 33.

## 2026-10-21 · A constant past a card is a compile-time error, always

- **Decision.** A constant no card holds, written or folded by rustc (`if x == 0 { 0 } else { 101 * 10 }`, where
  rustc hands the compiler 1010), stays refused, whether or not a run would reach it, with a message that says so in
  Rust's terms. The fuzzer counts such a program as one past a card, never as a finding.
- **Why.** It is what Rust does with overflow it can see: `30000 * 2` in an i16 is an error (`arithmetic_overflow`,
  deny by default) in debug and release builds, in a branch that never runs too, and `i16::MAX + 1` in a `const` is
  E0080. A card is the CARDIAC i16. Panicking at run time instead would put off an error rustc's own rules make a
  compile-time one.
- **Cost.** It is as good as rustc's constant folding, which depends on the opt-level: the same expression may be
  refused at one and panic as an overflow at run time at another. rustc's lint has the same limit.

## 2026-10-21 · Range patterns and consecutive arms cost what an if chain does

- **Decision.** Two changes to how a `match` is tested, both about comparison, what costs on CARDIAC. The range
  pass now settles a test written `k <= x` (the constant first, as rustc writes a range pattern's start) as it did
  `x >= k`, and the peephole drops a TAC straight after another TAC, which never jumps. And a switch whose
  arms are consecutive values (`0 => .., 1 => .., 2 => ..`, an enum's variants) is one subtraction, x - v0 below 0
  the other way, then `SUB 00; TAC arm` for each value in turn, where each arm was an equality, two tests.
- **Why.** A larger program rewritten in plain Rust: `match y { ..6 => 0, 6..13 => 1, 13..19 => 2, _ => 3 }` was 34 cells
  against 30 for the first-match form or the if chain; `match s { 0 => .., 1 => .., 2 => .., _ => .. }` was 39
  against 30 for `..=0 / ..=1 / ..=2`.
- **Cost.** The disjoint form is now 30 cells, the consecutive one 32: its first test (x below v0) stays when the
  ranges show x can't be, as the lowering doesn't see them.
- **Evidence.** `tests/programs/match_ranges.rs` and `switch_consecutive.rs` match native Rust.

## 2026-10-21 · A counter that only counts counts to 0

- **Decision.** After the range pass (`blocks::rebase_counters`), a local that is only set to constants,
  stepped by a constant and compared with one constant m (`c < k` read as `c < m`, `c <= k` as `c < k + 1`) counts
  from each constant minus m up to 0, and is compared with 0: `for _ in 0..96` is the `for _ in -96..0` a
  CARDIAC programmer writes. A loop `for _ in a..n`, n known only at run time, counts n down instead and drops its
  counter, the step first in the body where the test's subtraction is the new n: what `(a..n).rev()` compiles to.
  That needs n to hold the bound whenever the counter starts and to stay put while it counts: from the counter's
  block no way back to it but through a line that sets n, from such a line no way to the test but through that
  block, and from that block on nothing reads n but the test. So n may be a parameter, set on one arm of an
  `if`, set again before each run of an inner loop, or set before `overlay()`.
- **Why.** Comparison is what costs on CARDIAC: its one test is "below 0", so `c < 96` is a subtraction, a TAC and
  a cell for 96, where `c < 0` is the TAC alone. Every group of a larger program's machines, rewritten in plain Rust, found
  `for _ in 0..N` 1 to 3 cells over the hand-written loop, and wrote `-N..0` with a comment to say why.
- **Cost.** A counter whose value is read (`for x in 0..96` using x) is left as it is. So is one compared with two
  bounds, or set from a variable.
- **Evidence.** `for _ in 0..96 { out(k - inp()) }` is 14 cells, from 16, as `-96..0` is; `for _ in 0..n` is 11,
  from 14, as `(0..n).rev()` is. `tests/programs/count_loops.rs` and `count_down.rs` match native Rust.

## 2026-10-21 · Nested structs, and structs behind a `&mut`, are taken apart leaf by leaf

- **Decision.** A struct's field may be a struct, an enum with data (which the enum pass has written as a struct),
  or an array of scalars. The struct pass takes a struct apart into its leaves: its fields, with each field
  that is a struct its own leaves in turn (`Line { a: Point, b: Point }` is `l_a_x`, `l_a_y`, `l_b_x`, `l_b_y`).
  A place is one leaf's cell (`((_1.0: Point).1: i16)` is `l_a_y`) or, when it names a struct, a run of them,
  which a copy, a call's argument or a result copies cell by cell. A `&mut` to a struct is a `&mut` of each leaf:
  `(*r).x` is `*r_x`, `r = &mut p` is `r_x = &mut p_x; r_y = &mut p_y`, and passing r passes each.
- **Why.** It keeps the one rule of the struct pass: every later pass sees plain cells. It is what the enum pass
  already did for an enum within an enum, and it lets an enum carry a struct (`Option<Point>`) or sit in one
  (`struct Card { kind: Option<i16> }`) without either pass knowing about the other. A variant carrying a tuple of
  scalars (`Option<(i16, i16)>`) carries the struct such a tuple is written as, `tuple__i16_i16`.
- **Cost.** A `&mut` is an address a cell: each read or write through it is self-modifying code. Passing the
  values in and copying them back would be cheaper for a function that touches each field. An array within
  a struct can't be behind a `&mut` or in an array of structs yet, nor an enum in a struct in an enum.
- **Evidence.** `tests/programs/struct_nested.rs`, `struct_mut.rs`, `struct_array_field.rs`, `enum_field.rs`,
  `enum_field_mut.rs`, `enum_mut.rs`, `enum_payload.rs`, `enum_payload_array.rs` and `enum_payload_mut.rs` match
  native Rust; no other test program changed size.

## 2026-10-20 · A result is worked in its own cell

- **Decision.** `copies::share_cells` may merge a local into one of the function's result cells (the result keeps
  its name), under the same test as any other merge: never needed at once, with the caller's read after each
  return counted (`blocks::live_after_returning`). `Some(t)` at the end of a function that counts in t, or
  `(hi, lo)` built in locals, then counts in the result and the closing copy goes.
- **Why.** An `Option<i16>` returned as `Some(t)` cost a copy and a jump more than the same function returning a
  sentinel in `_0`; a larger program's trial of Option needs the two to cost the same.
- **Cost.** None measured: every test program that changed shrank (structs2 78 to 72, repeats 84 to 80), and no
  machine of a larger program grew (flee 88 to 86). The result cell lives longer, so the stack can't overlap it with a
  parameter that is still read; the local it replaces had that cell's lifetime anyway.

## 2026-10-20 · Struct constants, tuples across calls, and shared references as values

- **Struct constants.** A struct literal passed by value (`const Pair {{ .. }}`) is passed field by field, as a
  struct's cells are; the struct pass learns the fields from the constant when nothing else builds that struct. A
  promoted constant (`const main::promoted[N]`, rustc's `&P { .. }` or `&None`) is read in the IR
  (`Const::Promoted`) and built in the function that uses it, in locals of its own, from the promoted item's text.
- **Tuples across calls.** A tuple of scalars passed to or returned from a function that isn't inlined is a
  struct with fields `.0`, `.1`, ... (an overflow-checked op's `(T, bool)` stays a pair).
- **A shared reference is the value.** A `&Struct` is the struct by value: `*r` reads are the struct's
  cells, `r = &p` a copy. A shared reference to a scalar is passed as the value where every use is a `*r` read or
  an argument to a parameter of the same kind, and every definition a borrow or another such reference (a fixpoint;
  anything else keeps its address). Nothing can change behind a shared reference while it lives, so the value is
  the same. `addr_out` 41 to 33 cells; `addr_out_mut.rs` keeps the address path tested. `&mut Struct` is refused.
- **Derived `==` at opt-level 1.** rustc calls the impl's `eq` with a promoted constant: that is the promoted
  constant above; `ne` (core's own, whose body isn't compiled) is `!eq`; and the C-like pass no longer rewrites the
  enum's name inside a call's path (`<Colour as PartialEq>::eq` stays the function it names).

## 2026-10-20 · Fewer tests and copies: sign tags, jump threading, no self-copies

- **Why.** CARDIAC's one test is TAC, "jump if negative": `x == k` is a subtraction and two TACs, a `match` a chain
  of those, and a value tested once and then again pays twice. Most of what enums cost was tests.
- **Sign tags.** A two-variant enum numbered 0 and 1 with no niche (`Result<i16, i16>`, an `Option` whose payload
  may be negative) holds its number less one in the tag cell, -1 or 0: which variant it is, is the tag's sign, one
  TAC. A discriminant only switched on is that test; one read as a number is the tag plus one.
- **Jump threading.** A block that sets x to a constant and jumps to a block that only tests x (`switchInt(x)`,
  or `d = x op k; switchInt(d)`) jumps to the arm the constant picks, setting d itself; what the entry then no
  longer reaches goes. It is what rustc's own jump threading does at opt-level 2, and at opt-level 1 it undoes
  most of what the inlined methods cost (`checked_add` builds an `Option`, `unwrap_or` tests it at once). It also
  shrinks programs with no enums: `if c { 1 } else { 0 }` tested again, a loop's first test (the loop is rotated).
- **`==` of two `Option`s at opt-level 1** switches on each discriminant in turn (so each is a TAC) rather than
  comparing them as numbers. **`checked_add`, `checked_sub`, `checked_mul`** at opt-level 1 are the overflow-checked
  op and a test of its flag, as rustc inlines them at 2: `None` past a card.
- **Comparisons of two variables narrow both (range pass).** Where `a >= b` holds, a is at least b's lowest value
  and b at most a's highest (and so for each comparison, both ways), for i16 and u8. With it, `opt == Some(c)` and
  `Some(e) if e >= s` compare the payload first and test its sign after, which the ranges then decide; and
  `x.is_none()`'s `!(d == 1)` on a niche or a sign tag is the cell's own sign test, not a 0 or 1 worked out twice.
  A larger program's machines rewritten with `Option` for their -1 sentinels (stamp, gspan, and dist and sum reading to an
  end card with `while let`) are each the size of the original or smaller, their renders identical.
- **`x % k` and `x / k` with k past a card are x and 0.**
- **A sum's term read through an address comes first,** straight into the accumulator, rather than through a
  scratch cell.
- **No copy of a cell onto itself.** `STO x` right after `CLA x` (TACs and OUTs between) goes, in the peephole
  rules and once more after the compiled stack shares cells, which makes such pairs of a local and the argument cell
  it is passed in. Only that rule runs after the sharing: the others know a scratch cell's uses as they were before
  it (run whole, they dropped an argument stored for the multiply routine: the test programs caught it).
- **Evidence.** No test program grew; most shrank (try_result at opt-level 1 81 to 59 cells, result_methods 87 to
  51). A larger program's renders are identical to native Rust and no machine grew; 43 shrank, 91 cells in all.

## 2026-10-20 · An `Option<i16>` that is never negative has no tag: `None` is -1

- **Decision.** An enum shaped as `Option<i16>` (a plain variant and one carrying a single `i16`, numbered 0 and
  1) is laid out as its payload's cell alone when no payload it is given is ever negative: the plain variant is -1
  in that cell. `is_some()` is `payload >= 0`, a TAC; `x.unwrap_or(-1)` is the cell as it is; and
  `opt == Some(c)` tests `payload == c` first and `c >= 0` after, which the range pass decides when `c` is a
  counter from 0 (none is exactly -1, so the two orders agree).
- **Proved, not assumed.** Each payload written is followed by a guard, `assert(payload >= 0)`. The program is
  compiled with every such enum given the niche; an enum whose guard the range pass doesn't drop somewhere is laid
  out with its tag, and the program compiled again (`compile_niched`). A compile with niches that fails (too big,
  say) is done again with none. Guards proved cost nothing; the final compile has none left.
- **A test the ranges decide is a jump.** On its last visit the range pass turns a switch on `x op k`, with
  `x` an `i16` or `u8` whose range already decides it, into a jump: the facts it already trusts to drop asserts.
  (A `usize` past a card is held below 0, so its range isn't one to fold on.)
- **Not a `u8`'s.** -1 isn't a `u8`, and the range pass knows a `u8` is 0 to 255, so it would decide the payload's
  test wrongly: an `Option<u8>` keeps its tag.
- **Evidence.** A larger program's `boids/stamp.rs` written with `next_column: Option<i16>` (and `assume(c >= 0)` for the
  deck's columns) in place of `NONE = -1` is 64 cells and 54 instructions, as the original is, with the same
  output; `globe/target.rs` with `-> Option<i16>` in place of a sentinel function is 2 cells more (the callee's
  result copy). niche, niche_eq, niche_negative and niche_u8 match native Rust. The fold shrank several existing
  tests (compare_past 67 to 34 cells: `x < 1000` holds for every card) and grew none.

## 2026-10-20 · An enum with data is a struct of its cells

- **Decision.** An enum with data (`Option<i16>`, `Result<i16, i16>`, `enum Shape { Dot, Circle(i16), Rect(i16,
  i16) }`) is a tag cell, the variant's number, and cells for the fields, which the variants share: a field takes
  the first cell of its type that no earlier field of its variant has (`Shape` is three cells, `Option<i16>` two). A
  field that is itself an enum with data takes a run of cells, its own layout's, so what `?` builds
  (`ControlFlow<Result<Infallible, i16>, i16>`) works; a field of a type with no values (`Infallible`) takes none.
  `enums::data_enums` writes each such enum as a struct of those cells (`(_x as Some).0` is a cell, `discriminant(_x)`
  the tag, a variant built is the tag and its fields' cells), and the struct pass takes it apart as any struct, so
  passing and returning one needed nothing new.
- **The layouts come from rustc.** The driver writes down each enum the crate's locals have, per instantiation
  (`Option<i16>` and `Option<u8>` apart), each variant's number and its fields' types (`driver/src/facts.rs`). A path
  into std is core's: rustc names `Option` `std::option::Option` in a program with std.
- **Opt-level 1 and s.** rustc calls `is_some`, `unwrap_or`, `Try::branch` and the like rather than inlining them;
  `enum_methods` writes the code it would have inlined, as `methods` does for the integer methods. `unwrap()` and
  `expect()` on the failing variant call core's `unwrap_failed` and `expect_failed`, which never return: they are
  panics with Rust's messages. `map` with a closure is refused there: its closure argument isn't read yet.
- **Costs.** Within one function rustc's own MIR passes usually take an `Option` apart: a larger program's `globe/target`
  with an `Option` in place of its -1 sentinel is 62 cells to 61. Across a call the tag is a cell and a test or two,
  and the range pass doesn't know a callee's tag is one of its variants' numbers (tried: a `u8` tag, and assumptions
  where it is read; neither changed a cell, since the switch reads the callee's cell directly). The niche layout
  (`None` as -1 in the payload's cell) is next.
- **Evidence.** New tests (enum_consts, enums_for, option_methods, result_methods, try_option, try_result) match
  native Rust; the fuzzer generates enums (`--enums PCT`, 30 by default; `--enums 0` gives the old programs); no
  other test program's cells changed, and a larger program's listings and tapes are byte-identical.

## 2026-10-20 · The target is `cardiac-unknown-none`

- **Decision.** The notional triple, and the target directory `cargo cardiac` builds in, is `cardiac-unknown-none`
  (it was `cardiac-bell-none`): the machine, no vendor, no operating system.
- **Why.** rustc's target tier policy asks that a name not lead people "to form incorrect beliefs about what it
  targets": `bell` could read as Bell Labs' own or endorsed. The vendor field names who defines a platform (`apple`,
  `sony`, `vex`) or `unknown` (181 of rustc 1.97's 322 targets), and the OS field is `none` for bare metal. The
  policy also warns that a name is disruptive to change once it spreads: changed now, before binaries are published.
  rustc never sees it (an official target needs a backend in rustc itself), so it is a name only.
- **Not a product's name.** The first field is what the compiler compiles to, and the promise is standard CARDIAC,
  the 1968 machine as Bell Labs specified it: a tape runs on any faithful CARDIAC (the C# one, the flipboard, the
  cardboard by hand). A CARDIAC product of ours is one implementation of that platform, not its vendor; anything a
  program needed beyond the standard machine would break the promise (today nothing does: the tape is cards the
  machine's own bootstrap loads, and the panic map is a side file it never reads).

## 2026-10-20 · The driver is the backend

- **Decision.** The CARDIAC driver compiles a program for CARDIAC in rustc's process. `cargo cardiac` runs
  `cargo check` with the driver as cargo's `RUSTC` (or, for a single file, the driver itself), and tells it what to
  make in `CARDIAC_BUILD` (JSON: `build` or `test`, the programs by source file, each with its name, its source as
  shown and its resources, and the crates they use); the opt-level and debug assertions are rustc's session's. When
  rustc has analysed a crate given `--cfg cardiac` whose root is one of the programs, the driver converts its MIR to
  the IR, converts the MIR of each crate it uses from that crate's metadata (they are built with
  `-Zalways-encode-mir`), links them (`crates::link`, now on the IR, its renaming rules as they were), compiles, and
  writes the listing, the tape and the panic map (or, for `test`, FILE.mir and FILE.asm) where `cargo cardiac`
  wrote them. It says what it made in a line of JSON on rustc's standard error, which cargo hands on as one of the
  program's messages; `cargo cardiac` prints from it what it printed before (the sizes, `-v`'s sections, the
  notes) and reads the tape and map back for `run`. A program that doesn't compile for CARDIAC is a rustc error with
  the text `cargo cardiac` prints (`error: FILE: ...`), so rustc fails, and cargo's build with it; `cargo cardiac`
  renders rustc's other diagnostics itself, as cargo would, and says that one as before. A program's crate
  depends on `CARDIAC_RUN` (rustc's env-dep), new for every run, so cargo compiles the programs every time: their
  outputs are not cargo's to track, and the compiler or a resource may have changed. No `.mir`, `.cardiac.mir` or
  `.cardiac.json` is written or read on the way; `program::read_mir` and its mtime rule, `single_mir`,
  `enums::beside` and the facts file are gone.
- **Another crate's names.** The driver converts a dependency in the *program's* session, so rustc names its
  items as the program sees them (`kit::tables::SQUARES`, `<Point as Clone>::clone`), and the crate's name is taken
  off the front of its own items' definitions and calls. A first version renamed them back to what each crate's own
  build would have printed, by modelling rustc's printer over the crate's metadata, to keep listings byte for byte;
  that was dropped: names need only agree within one build, and what the compiler is held to is its output, not
  its labels. An impl's function in a dependency is converted only if a call reaches it (a derived `Clone` no call
  makes needn't compile), as only reached ones were before, when the text named the others `<impl at ..>`. A
  dependency's statics have no MIR in its metadata, so they are not in the IR (the compiler refuses statics).
- **The fallback.** Dropped: the driver is required. Plain rustc could only give the program to the compiler as
  MIR files, the hand-off this replaces, and keeping that path meant keeping its parser input, its mtime rule and
  the text linker beside the new ones, for programs that already lost enums and methods without the
  driver. Without a driver `cargo cardiac` says how to build one (`cd driver && cargo build --release`; rustup
  installs the pinned nightly from `driver/rust-toolchain.toml`).
- **How it was checked.** The oracle (`CARDIAC_IR_CHECK`, `ir_check.sh`) checks each program's own IR against
  rustc's text as before; each dependency's items, converted from metadata, against rustc's text of the same
  bodies; and the same items as the view names them against the IR the dependency's own build converted (kept,
  under the oracle only, beside its report): 44,576 reports, 261,092 items, all equal, over the test programs, the
  two-crate package's six profiles, the examples' three, the fuzzer's programs and 300 generated ones, and a larger program of 109 machines. Every listing of the test programs (opt-level 1 and 2,
  checks on and off), the examples, the two-crate package and that larger program is byte-identical to main's; test_compiler.sh
  334 MATCH, test_examples.sh 45 SAME, the fuzzer 0 FAIL and 300 generated programs 0 differ (13,840 runs, as main),
  the larger program's references 3146/3146. A clean build leaves no `.mir` or `.cardiac.*` file in any target directory.
- **Cost.** The view is a model of rustc's printer for another session, kept in step with it by the oracle, and it
  can be wrong where metadata is silent (a glob import a dependency uses). A program's crate is checked again on
  every build: the larger program's 109 machines take 0.36 s with nothing changed (main 0.21 s), 0.40 s clean (main 0.52 s). A CARDIAC error now also gets cargo's own line (`error: could not compile ...`)
  before `cargo cardiac`'s, and a panic of the compiler's is rustc's thread's (`thread 'rustc' panicked`), reported
  as an error, not a crash of `cargo cardiac`. A dependency rustc didn't load for the program (one it never names) is
  not linked, so its names no longer count when two crates' functions clash.

## 2026-10-20 · An impl's function is named as its calls name it

- **Decision.** In the IR the driver builds, a function in an impl (inherent or a trait's) that a call in its crate
  reaches is named as that call names it: `P::sum`, `<P as Twice>::twice`, rustc's printer without the
  `<impl at FILE:L:C: L:C>` the text gives it. Each call is resolved to the definition it runs
  (`Instance::try_resolve`, a trait's function to the impl's) to know which. The front end makes
  `<P as Twice>::twice` one plain name, `P_as_Twice__twice`, at the definition and at each call, as it does
  `kit::arith::twice`. One no call reaches keeps the text's name, and the compiler leaves it out, as before; an
  impl's const keeps it too (the front end knows a const's owner by it).
- **Why.** The text names a method's definition by where its impl is and its calls by its type, so no call could be
  matched to its definition: a method rustc didn't inline (`#[inline(never)]`, or opt-level 1) was refused as a
  call to an unknown function.
- **Measured.** No listing changes: in the test programs, examples, fuzz programs and a larger program no other function is
  renamed (the oracle counts the renamed, and compares them under the text's names: 12, all in
  `tests/programs/methods.rs`). `methods.rs` builds at opt-level 1 and 2 and prints what native Rust does.
- **Cost.** A derived `==` on a C-like enum at opt-level 1 is still refused: its call passes a promoted constant
  (`&Colour::Green`, `const main::promoted[1]`), which the IR doesn't read, and its definition takes references.

## 2026-10-20 · The compiler reads the IR the driver builds from rustc_public

- **Decision.** The CARDIAC driver builds the compiler's IR (`ir::File`) from rustc_public's MIR
  (`driver/src/convert.rs`) and writes it, printed as MIR (`ir::File`'s `Display`), beside rustc's text:
  `NAME.cardiac.mir`. The compiler reads that in place of `NAME.mir` when it is there and current
  (`program::read_mir`, for single files and packages alike); without the driver, or for a crate it couldn't
  convert, it reads rustc's text as before. The IR's structure is rustc_public's: items, locals and the shapes of
  their types, statements, places, operands, rvalues, terminators, switch arms, assert messages, and integer, bool
  and unit constants from their bits. Its names are rustc's printer's, through `rustc_internal`, as the text's are
  (trimmed paths depend on resolver state rustc_public doesn't show), and so are the constants, lines and items the
  IR keeps as text; three facts rustc_public leaves out come from rustc's MIR (a local's scope, which orders the
  declarations; a coercion's source; why an unwind terminates).
- **Why.** The next fixes need rustc's facts, not its printer's text: a call resolved to the definition it reaches,
  a variant's number. The IR the compiler works on now comes from them, and a fact can be added to it
  without a new syntax to parse.
- **How it was checked.** An oracle (`CARDIAC_IR_CHECK`, `ir_check.sh`): the driver also makes rustc's text as
  `--emit=mir` does and compares the IR the compiler parses from it with the converted one, item by item, and
  checks that both print and read back the same. Over every test program (opt-level 1 and 2, overflow checks on
  and off, debug assertions), the two-crate package's six profiles, the examples' three, the fuzzer's programs and
  300 generated ones in eight settings, and a larger program's machines: 3,386 crates, 16,872 items, all equal. Where the
  parser reads the text oddly (`u8::MAX` is 255 but `i16::MAX` a name, a switch arm its bits, `Cmp(a, b)` an enum
  variant, `(copy _1,)` and a unit struct not read), the conversion does the same, each said where it is matched.
  Every listing (test programs at opt-level 1 and 2 with checks on and off, examples, the two-crate package,
  a larger program) is byte-identical to the text-reading compiler's.
- **Cost.** The conversion has to be kept in step with the parser while both exist, and the oracle is what keeps
  them so: a construct the parser learns must be converted too. The names, and constants other than scalars, still
  pass through rustc's printer.

## 2026-10-20 · Into the compiler: rustc as a library, on a pinned nightly

- **Decision.** A driver (`driver/`, the binary `cardiac-driver`) is rustc built as a library (`rustc_driver`, and
  `rustc_public` to look at what it compiled), on `nightly-2026-05-22`, the last of 1.97's nightlies. It runs as
  rustc does, configured as rustc's own driver configures itself, so its MIR prints byte for byte the same; and
  where it writes a crate's MIR (`NAME.mir`) it writes `NAME.cardiac.json` beside it, what the text leaves out.
  First the enums the crate's functions use, with each variant's number. `cargo cardiac` runs it in place of
  rustc, for a single file and as cargo's `RUSTC`, when it finds it (`CARDIAC_DRIVER`, beside itself, or this
  repository's `driver/` build), with the nightly's libraries from rustup; otherwise plain rustc, as before.
- **Why.** The MIR text is rustc's printer's, for people ("subject to change without notice"), and several bugs
  were what it leaves out or prints oddly: switch arms as unsigned bits, 255 as `u8::MAX`, a call of
  core's whose debug-assertions setting it doesn't show, a method's definition and calls under different
  names, and enums' numbers, which it never gives. Inside rustc each of those is a fact to look up. This
  replaces part of 2026-10-12's "Take MIR from stable rustc": the cost it named, "a backend inside rustc is the
  long-term fix", is being paid. The text's own pin to 1.97 (its format changes between releases) is a pin
  still, to the nightly of the same MIR: the compiler's tests, examples, fuzzer and a larger program are byte-identical
  through the driver, the compiler as it was the oracle.
- **Cost.** The nightly (rustup installs it from `driver/rust-toolchain.toml`) and its `rustc-dev` component, and
  porting the driver when the pin moves: `rustc_public` changed between May and October 2026 (`Crate::adts`, how a
  variant is numbered). Next: the IR from `rustc_public`'s MIR rather than the text, checked equal to the text's.

## 2026-10-20 · The build's debug-assertions setting reaches the compiler

- **Decision.** `Mir` carries whether rustc built the program with debug assertions on: from `-C
  debug-assertions` for a single file (rustc's default otherwise: on at opt-level 0, off above it), from cargo's
  metadata (`profile.debug_assertions`) for a package. With it on, the precondition pass makes a call of
  `core::hint::assert_unchecked` itself the same check as its inlined `precondition_check`: Rust's message, exit
  134. With it off the call stays the program's promise.
- **Why.** At opt-level 1 and s rustc doesn't inline `assert_unchecked`, and its body, which checks the condition
  when debug assertions are on, is core's: no crate's MIR carries it, and the caller's MIR is the same either way.
  So a false condition with debug assertions on printed a value where Rust aborts.
- **Measured.** No listing changes for a build with debug assertions off; precond_assume and xhint, which differed
  at opt-level 1 with them on, match.

## 2026-10-20 · Core's checks of unsafe preconditions are asserts

- **Decision.** With debug assertions on, rustc keeps the checks core makes of an unsafe function's precondition
  (`ub_checks`): `x.unchecked_add(y)` first calls `unchecked_add::precondition_check(x, y)`, a function of core's
  that no crate's MIR carries. The front end (`preconditions`, before anything else) makes each such call an
  assert of what it checks, with Rust's message, which the panic map gives: unchecked_add, _sub and _mul an
  overflow-checked op and its flag (`usize::unchecked_sub cannot overflow`), unchecked_neg `0 - x`,
  unchecked_shl and _shr the shift below the bits, get_unchecked and get_unchecked_mut the index below the length,
  assert_unchecked its condition, unreachable_unchecked `assert(const false)`. One the test before it proves is
  dropped there: a `for` loop's step, which rustc's inlined `next()` takes only after `start < end`, so
  `unchecked_add(start, 1)` (and `end - 1` of a reversed range) can't overflow; a signed range's overflow way into
  unreachable_unchecked's check is `unreachable`, as it is without debug assertions. Natively a violation is a
  panic that can't unwind, and the process aborts (SIGABRT, exit 134); on CARDIAC it punches -999 and halts as
  every failed check does, and `cargo cardiac run` says why and exits 134 too. `-C debug-assertions=on|off` is
  a single file's flag, as rustc's. test_examples.sh builds and runs every example with the `checked` profile as
  well, both ways.
- **Why.** The `checked` profile couldn't build `primes`, `sort`, `cipher` or `powers`: every `for` loop over a
  range has such a check in it. A check dropped unasked would be a wrong answer where Rust aborts (dropped,
  `3usize.unchecked_sub(5)` is -2, as it is without debug assertions), so each is kept unless proved; and only
  the step a loop's own test proves is dropped before the passes, because the for-loop pass needs the step in
  the block that tests, as rustc writes it without debug assertions. The rest the range pass proves or keeps, as
  for any check. One machine, one way to stop: the -999 card is the panic either way; the exit code is the
  host's, and it says what `cargo run` says.
- **Measured.** Every example builds with the `checked` profile, the same size as with `dev` (`days_assumed`
  aside, whose `assume` is checked there: 38 cells). No listing of a test program, an example's `dev` build or a
  larger program's machine changes. `precond_loops` (each kind of `for` loop) is the same code with debug assertions as
  without.
- **Cost.** A check the passes can't prove costs a few cells (`a.unchecked_sub(b)` of two parameters: 4), and a
  proved overflow-checked op leaves its sum's dead store where the passes don't remove one (a field of a
  tuple). At opt-level 1 rustc doesn't inline `assert_unchecked`, and the compiler, not told of debug
  assertions, takes the call as a promise, unchecked (unchecked_add and its kin are refused there). The checks of
  get_unchecked (on a slice), unwrap_unchecked (an Option), unchecked_mul and the shifts (no lowering of their
  ops) are made, but those programs are refused for their types or ops all the same.

## 2026-10-20 · Natively, the panic hook is set up before main

- **Decision.** The built-ins crate's native half puts a pointer to its setup in the executable's section of
  constructors (`__DATA,__mod_init_func` on macOS, `.init_array` on other Unix, `.CRT$XCU` on Windows), which the
  loader runs before `main`, as it runs a C++ global's constructor. The setup is the panic hook that prints -999. The
  first `inp()`, `out()` or `load_resource()` still sets it up (once), for any other target.
- **Why.** Stable Rust has no way to run code before `main`, so the hook was set up by the first `inp()` or
  `out()`. A program that panicked before either printed nothing natively and -999 on CARDIAC, and the fuzzer and
  native comparisons called that a difference. Teaching them that "nothing, exit 101" matches "-999, exit 101"
  would have been a special case in every comparison; this makes the native build do what CARDIAC does.
- **Cost.** A `#[used]` static in a `#[link_section]` (no `unsafe`). Checked on macOS in debug, release and with fat
  and thin LTO, for a program that names nothing in the crate.

## 2026-10-20 · Code that runs off its end brings the next unit with it

- **Decision.** When main's last phase, or a function or routine, ends in an instruction that carries on to the next
  cell (not HRS, not a JMP that isn't a call), the unit after it comes with it: `split_phases` puts it straight after
  the phase's code whether or not anything names it, and keeps a unit that runs into the next as one unit with it.
  Peephole rule 6 keeps the `rt_panic` label while the line before runs into it. With those, rule 4 drops every JMP
  to the next label, `rt_panic`'s too, and the earlier exception (keep the last branch to `rt_panic`) is gone.
- **Why.** A phase takes in a unit only when its code names it. Jump threading, rule 4 and the label sweep can leave
  main reaching the panic routine only by running into it: at opt-level 2, `for i in 0..0 {..}; out(0); out(0 / 0)`
  lost both branches to `rt_panic` (the exception kept one only while it was the last), then the label, so the
  routine's two instructions stayed in main's code unlabelled; the panic map had no stop, and `cargo cardiac run`
  exited 0 after the -999. Keeping the label alone made it worse: the routine became a unit nothing named, and was
  left out. A function always ends with its `_exit` cell, so none runs into the routine today; the rule covers it.
- **Measured.** A program whose panic main runs into is one cell smaller (the exception's kept JMP), and the program above one
  larger (12 cells: block layout keeps the piece that runs into the routine last), where it exited 0.
- **Amended.** No instruction at all before a label counts as running into it: the program starts there. A
  main that is only `panic!()` lost the label and exited 0 again. Its `JMP rt_panic` stays too, though the routine
  comes next, as does any whose check no line before it can take (none, or one with a check of its own): the panic
  map gives a run's reason by the line that ran into the routine, and without one it said only "a check failed".

## 2026-10-20 · A usize past a card is held below 0

- **Decision.** `-1i16 as usize` is usize::MAX in Rust, and no error; without overflow checks `0usize - 1` is too.
  Such a usize, past a card, is held as the negative number it is short of 2^64 by (-1 for usize::MAX), in the
  locals `ranges::huge_usizes` finds can hold one: those an `i16 as usize` or a usize difference that can go below
  0 (a `SubWrapping`) is stored into, and those a copy, sum or difference of one is. Not `_0`, a struct
  result's field, or a local whose address is taken: a caller or a pointer reads those as a usize a card holds. The
  cast no longer panics. What is done with one: a comparison tests its sign first, since such a usize is above
  every usize a card holds, and then compares as before (one side: a load and a TAC; both: two of each and
  jumps); `as i16` is the number held; `as u8` adds 256 while it is negative before taking 256 away; a sum or
  difference into another such local is the plain op, exact modulo 2^64; as an index, the bounds check (a
  comparison) fails, as Rust's does. Anything else, passed to a function, stored where a card's usize is read
  (`_0`, an element, a field), multiplied, divided, or an overflow-checked op, panics there first: "the value
  doesn't fit on a card". The range pass gives these locals -999..=999 where it knows nothing better, settles no
  comparison of one that can be below 0, and narrows `x < c` to 0..c-1 but `x > c` not at all when x can be below
  0. Whether a difference can go below 0 depends on which usizes can, so the analysis runs from none and again
  until the differences it finds can go below 0 are all counted.
- **Why.** The cast panicked as if a bounds check followed (`out((m - 1) as usize as i16)` printed -999 for 0,
  where Rust prints -1), and an index made from `0usize - 1` without checks was -1, which passed `j < 4` and read
  the cell before the table. Rust's answer is exact on a card for what a usize past a card is used for in practice:
  compared, cast back, indexed with. Where it isn't (Rust's value is past 999 or panics on overflow, which no card
  shows), it panics as a value past a card does with overflow checks on, never a wrong number: with checks on,
  any checked op on such a usize either overflows in Rust or gives another usize past a card.
- **Measured.** checked_cast, without overflow checks, 76 to 79 cells (its `(i + 254) as u8` of an i + 254 that
  can be past a card); with them the panic moves from the cast to the bounds check, the same size, as in the
  examples' days and cipher. In a larger program: ray's compose 33 + 26 / 42 to 32 + 25 / 42 cells (data + phase 1 / phase
  2: its `inp() as usize` count no longer needs the panic routine in phase 1), flight's wave_add the same 81 with
  its check moved; every frame of every effect and the cut identical to native. No other listing changes.
- **Cost.** A comparison of a usize that can be past a card costs its sign test, a local it is stored in can't be
  carried in the accumulator, and a use that can't hold one costs a load and a TAC. A usize past a card that
  is passed to a function, multiplied or divided panics where Rust carries on.

## 2026-10-20 · A u8 wraps, as Rust's does without overflow checks

- **Decision.** Before any pass, `ranges::mark_wrapping` makes each `+`, `-` and `*` rustc writes on u8 (with
  overflow checks off, or an inlined `wrapping_add` and its kin) its wrapping op, `AddWrapping` and its kin. The
  range pass turns one back into the plain op where the operands' ranges keep the result within 0..=255; one it
  can't lowers to the plain op and then the wrap in the accumulator: a difference of two u8s gets 256 added back
  when it is negative (3 instructions), a sum or product has 256 taken away while it lasts (4). The ops the passes
  write themselves (a `for` loop's step, `saturating_sub`'s difference) wrap nothing and stay plain. Beside it the
  range pass now narrows on `switchInt(copy n) -> [0: .., otherwise: ..]`: n is not 0 on the way that steps, so a
  u8 counted down to 0 needs no wrap (and, checked, no check).
- **Why.** A cell kept `0u8 - 1` as -1, which no u8 holds: `c < 8` passed and `T[c as usize]` read the cell before
  the table, where Rust's bounds check panics at 255. Every u8 now holds 0..=255, so its comparisons and indexes
  are Rust's.
- **Measured.** Unchecked, where the ranges can't keep the result a u8: add 30 to 35 cells, nested 45 to 50, fib
  17 to 22, fib_overflow 12 to 17, arrays 52 to 60, refs 75 to 80, div_by_own 43 to 48, divide 38 to 42,
  forloops4 63/62 to 67/66 (opt-level 1/2), underflow 6 to 10; add, fib_overflow and underflow print what Rust
  does now where they printed 280, -1 and the like. Checked, countdown 12 to 8 (the new narrowing proves its
  `n -= 1`). The examples' and a larger program's listings don't change.
- **Cost.** A wrap where the ranges can't follow a value: a u8 parameter (`k * 2` in a function of its own), a
  sum through a reference, a counter stepped by 2 to 0. A u8 product past 999 is past a card: the cell has kept
  its low three digits before the wrap.

## 2026-10-20 · A check rustc has proved fails is the panic

- **Decision.** A pass before all the others (`blocks::settled_checks`, at every opt-level) takes an assert on a
  constant, `assert(const false, ..)` or `assert(!const true, ..)`, as rustc writes them for a program that
  `#[allow(unconditional_panic)]`s (an index past the end by constants, `x % 0`, `T[-1 as usize]`). One that holds
  is a goto; one that fails keeps no statements and goes on to itself, so the blocks after it are reached from
  nowhere and are dropped, with the statements that only fed them. It lowers to `JMP rt_panic`, marked with the
  check's reason for the panic map. (Beside it, peephole rule 4 kept the last branch to `rt_panic` even when the
  routine came next; running into the next unit replaced that.)
- **Why.** Native Rust panics there, so CARDIAC must too, with the same reason; it refused them instead
  ("unsupported place const false"). Once the program has panicked nothing it stored can be seen, so the failing
  block's statements need not run: `T[K as usize]` with `K = -1` stores `usize::MAX`, past a card, only for the
  bounds check that fails, and is never stored now. Rule 4 dropped a branch to `rt_panic` that the routine
  followed, and with no other branch left the routine lost its label and the panic map its stop: a program whose
  only panic ran straight on into it (`out(x); panic!()`) exited 0 from `cargo cardiac run`.
- **Measured.** No listing of a test program, an example or a larger program's machine changes; settled_oob, settled_rem and
  settled_neg, refused before, are 6 to 25 cells. No program the fuzzer makes from seed 22 is refused this way now.
- **Cost.** One cell (the kept `JMP rt_panic`) in a program whose one check falls into the panic routine.

## 2026-10-20 · Code rustc copies into both ways of a test is joined again

- **Decision.** A pass after the range pass (`tails::join_tails`) finds a two-way branch whose two ways, each
  reached only from it and followed block by block through gotos, asserts and calls, run the same steps somewhere
  in between (the same statements, checks and calls, in the same order, on the same locals). Those steps go into
  blocks of their own that both ways go on to, each way keeping what it ran before them; the shared blocks end by
  making the branch's test again and going on to what each way ran after. Only when the test can be made again
  (its comparison's operands, or its bool, are whole locals whose address is never taken and which nothing on the
  way writes), and only when the shared steps are worth at least 8 cells by a rough count (a call to the multiply
  or divide routine and something more). Beside it, a peephole rule (14): a block that starts `CLA x` where every
  way in, each jump to it and what falls into it, has just stored or loaded x, loses the `CLA`. It is tried only
  once the other rules are done, since it takes away the `CLA` rule 10 starts from.
- **Why.** Once `abs(v)` is inlined, rustc answers the second `v < 0` from the first and copies `abs(v) * 10 / z`,
  zero check and all, into both ways (jump threading): two calls of the division routine, each with its argument
  stores and return, where testing the sign again is a load and a TAC. Each way still does the same work in the
  same order from the same values, so the join is sound when the second test gives the first one's answer. After
  the range pass, the checks each way's own facts proved are gone already, and only code both ways still run is
  joined: no check comes back (the zero check after a promise and the `MIN / -1` one stay dropped). The joined
  code starts by reloading what both starts stored, which rule 14 drops; it drops the same reload after any if/else
  whose arms both store what follows reads, and at a loop's head that reloads what the loop just stored.
- **Measured.** The program that showed it (`inp()`, `assume(z >= 1)`) at opt-level 2 without overflow checks: 102 cells
  to 95 (101 by rule 14 alone), so it fits. The fuzzer's `branches`: 81 to 78 there, and 110 to 101 at opt-level
  1 unchecked. `joined_tails` (a multiply by a local, its overflow check shared) 92 to 84 at opt-level 2 checked;
  `joined_tails2` 72 to 68 unchecked. A divide alone is not joined: it measured 2 to 3 cells more (the second
  test, a jump, and the test's operand kept apart from the start's result), so `divide_branches` is 90 to 89, by
  rule 14 only. Rule 14 takes 78 cells off 47 of the 110 machines of a larger program, 1 each off cipher, primes, sort and
  both phases of powers, and 1 to 3 off 17 test programs; nothing anywhere gets bigger. The join itself finds
  nothing in that program. Every frame of it, built with both, is still identical to its native reference (3146
  of 3146 in the cut).
- **Cost.** The ways now run the test twice, a load and a TAC more each time through. The worth count is rough:
  it may join a run that costs a cell or two, or leave one that would pay, and a way's steps are matched only in
  the order they come (a run that shares its steps in another order is left).

## 2026-10-20 · `assume()`, an unsafe fn, replaces `promise!`

- **Decision.** `cardiac::assume(cond)` is an `unsafe fn`, called as `unsafe { assume(cond) }` with a `// SAFETY:`
  comment saying where the guarantee comes from. On CARDIAC it is `core::hint::assert_unchecked`, no code, unless
  debug assertions are on (`debug-assertions` in the profile), where it is an `assert!`: a false assumption punches
  -999 and halts. Natively it is always an `assert!` (`#[track_caller]`, so the panic names the line). The
  `checked` profile builds the examples that way. `promise!` is gone.
- **Why.** `promise!` hid its `unsafe`: safe-looking code whose false condition is undefined behaviour, which Rust
  calls unsound. Rust shows undefined behaviour at the call site, and its own `assert_unchecked` is an unsafe fn
  that checks in debug builds; `assume` does both, the name C++ (`[[assume]]`) and LLVM (`llvm.assume`) use for
  "trust this, generate nothing". `promise!` also didn't build with debug assertions on: rustc puts a precondition
  check into `assert_unchecked` there, a function the compiler doesn't know.
- **Measured.** The default build is unchanged: a test program with three assumptions is the same 72-cell tape
  with `assume` as with `promise!`. `days_assumed` (the `days` example assuming a month 1..=12) is 21 cells to
  `days`'s 31. Checked, it is 64: the checks are about 3 cells each, and the rest is the month table, which is
  built at run time when a check comes before it instead of being loaded with the tape.
- **Cost.** Two `unsafe` lines where there was one macro call. A checked panic's reason is "explicit panic", not
  "a broken assumption", as for every `assert!` message.

## 2026-10-19 · A compiled stack: data cells shared by liveness

- **Decision.** After the peephole rules, a pass (`stack::share_cells`) works out where each data cell is live
  across the whole program, functions included (a call goes to the callee's entry; its return comes back after
  every call to it), and packs cells that are never live at once into one, the first in the listing standing for
  the rest (its line notes them: `also main_8 (never live at once)`). Both halves at once: a loop
  counter used early with a total used late, and the cells of two functions never active together. Accesses stay
  plain direct addresses. A cell takes part only if every use is a plain read or write (CLA, ADD, SUB, OUT, STO,
  INP) and something writes it: constants and address templates (never written), arrays and resource slots
  (reached through their address), cells named in a `DAT` or as `name+N`, and exit cells keep their own. A
  program in phases, or with a jump the pass doesn't follow, is left as it is.
- **Why.** Measured first: of 1477 such cells in the 266 one-phase reference cases, 508 could go. Built: 194 cases
  change, every one smaller, 580 cells in all; the reference machines lose 167 between them (`flee`, at the
  96-cell limit, 6); a test program that didn't fit now does. Examples: gcd 83 to 80, sort 77 to 76, cipher 57 to
  56, days 32 to 31. Every changed tape was run old and new on 300 random decks each, decks that keep the
  machine's promises (`promise!`): identical (55,800 runs). Decks that break them read past an array, and what is
  there depends on the layout, so they differ, as undefined behaviour may. Planting a bug in the pass (a TAC's
  target left out of the flow) makes 21,638 of those runs differ.
- **Cost.** Phases aren't shared yet (each phase's data is split already, and they were left out of scope). The pass
  packs greedily, the most constrained cells first: not always the fewest cells, but deterministic.

## 2026-10-19 · A comparison against an array element is mirrored, not parked

- **Decision.** In an ordering comparison (<, <=, >, >=) of two values where one comes through the accumulator (an
  array element, read through its address), that one is loaded and the other, in its cell, subtracted. When the
  usual form would subtract it instead, the comparison is mirrored: `x < y` as `y - x - 1` not negative, `x <= y`
  as `x - y - 1` negative, the 1 subtracted from cell 00, which always holds 001. When both come through the
  accumulator, the one subtracted is parked in a scratch cell. Equality and comparisons with a constant are as
  before, and so is a value carried in the accumulator from the statement before (it is never loaded over).
- **Why.** CARDIAC subtracts only a cell. Parking the element and loading the other side cost a store, a load and a
  scratch cell where the mirror costs one SUB. Found by measuring each pass switched off: `sort` ran faster without the
  first `forward_copies`, which had left `xs[i] < xs[min]` with the element on the right.
- **Cost.** None: three reference cases change, each 2 cells smaller (`golden-pre-mirror/` has the old ones), and
  `sort` goes from 80 cells and 2356 instructions to 77 and 2188. `compare_left`, `compare_right` and
  `compare_both` check every ordering against native Rust, with negatives and equal pairs.

## 2026-10-19 · No profile-guided optimisation: measured, no pass ever trades speed for size

- **Decision.** No PGO. The passes run as they do, the same for every program.
- **Why.** PGO pays when a transformation helps some programs and hurts others, so choosing per program, by
  running each candidate, beats always doing it. Measured first: each of the 14 IR passes switched off in turn,
  over the examples with their test inputs (15 runs) and the test programs at opt-level 2, with and without
  overflow checks (83 that finish), against all passes on. In 97 of the 98 runs no pass is ever better off, in
  instructions run or in cells: every pass is neutral or a win. The one exception is `sort` with the first
  `forward_copies` off, better on both counts (2356 instructions to 2170, 80 cells to 77): not a trade-off for a
  profile to choose, but a pass that does worse on one program, to fix in the pass. Measured inlining, the
  first candidate, was already measured and set aside.
- **Cost.** None built. If a transformation that trades speed for cells is added, measure again.

## 2026-10-19 · A panic map says why a run panicked

- **Decision.** `cargo cardiac build` writes a panic map, `NAME.map`, beside the tape: the cell of each branch to
  `rt_panic` (or of the line that runs on into it) with what it checks, as Rust says it (`index out of bounds`,
  `attempt to add with overflow`; a runtime routine's own check, `the value doesn't fit on a card`), where each
  function starts, and where `rt_panic` is, for each phase. A host runs the tape keeping track of the last three
  instructions run (`cardiac::run_traced`); a run that halts in `rt_panic` (its OUT, then its HRS) panicked, and
  the instruction before them is the check. A runtime routine's check is reported in its caller, found by the
  routine's return jump (cell 99 for `rt_chk`, the routine's exit cell for the others). `cargo cardiac run` prints
  `panicked in main: index out of bounds` on standard error and exits 101; the flipboard says it in its caption,
  given the map with the tape; in Rust, `panics::Map::panic`. The check travels from lowering to the map on
  `Line::panic`, which is never printed and is no part of a line's equality: listings and tapes are unchanged.
- **Why.** The first proposal was to read cell 99 after the halt, but a TAC to `rt_panic` doesn't write cell 99, and 60
  of the reference cases have one: each would have needed a JMP of its own, a cell, and a full program would no
  longer fit. Keeping track on the host's side costs the machine nothing, covers TAC, JMP and running on into
  `rt_panic` alike, and leaves a real CARDIAC as it was: -999 and a halt. A program that prints -999 itself and
  halts elsewhere is not taken for a panic.
- **Cost.** No source line: stable rustc's MIR has none for a check, so the report names the function. The values
  (the index, the length) aren't known either, as the map is written before the program runs. Tracking runs only
  with a map, and costs nothing measurable even then (325 million instructions in 0.45 s, as before).

## 2026-10-19 · No inliner of our own: measured, it would rarely pay

- **Decision.** The compiler doesn't inline (plan step 6, measured inlining). rustc's inlining at opt-level 2 is
  the only inlining, and `#[inline(never)]` keeps a call.
- **Why.** Inlining pays when a call's overhead (about 5 cells: the return address kept, the exit cell, the
  return jump, the call) is more than the body it copies. Among the 466 reference cases, setting aside every
  function marked `#[inline(never)]`, five helpers called once would save about 5 cells each, nearly all at
  opt-level 1, where rustc doesn't inline; at opt-level 2 rustc has already inlined what is small. The two
  real-world helpers rustc keeps as calls lose when inlined: one, with two loops and two call sites, would grow
  its program by about 60 cells past the 96 it has; the other grew its program from 61 cells to 66. And
  `#[inline(never)]` isn't in the MIR, so respecting it would mean parsing every program's source too.
- **Cost.** A program built at opt-level 1 keeps a call to a helper it calls once (about 5 cells). Build at
  opt-level 2, or mark the helper `#[inline(always)]`, where rustc will inline it.

## 2026-10-19 · lin_fold stays within a block: measured, it would never fire

- **Decision.** lin_fold is not extended across blocks (plan step 5). It folds within one block, as before.
- **Why.** Measured first, as planned: a temporary of plain Add, Sub, Neg or a small Mul, used once in a later block
  of a straight run (a goto, an assert or a call's return between, the later block entered only from the one
  before). Across the 466 reference cases, every test program at opt-level 1 and 2 without overflow checks, and
  all nine examples, there are none. `merge_blocks` already joins the gotos just before lin_fold, so only asserts
  and calls are left between, and code that starts a sum, divides or calls, and then finishes the same sum doesn't
  turn up. The plan's own example, `c * 3 - (a / d + b) * 2`, does fold (3 cells), and only without overflow checks.
- **Cost.** None: nothing is built. If a program of that shape turns up, measure again.

## 2026-10-19 · A resource is loaded once: in main, outside any loop

- **Decision.** `load_resource` is a compile error outside `main`, and on a loop in `main` (a block a path from it
  leads back to), at every opt-level. With the rule already there, one load of each resource in the whole program,
  a resource's load runs exactly once. One slot per resource stays, and the program may write to the array after
  it is loaded: as scratch, or anything else. A second copy is `let copy = key;`, cells of its own.
- **Why.** The loader fills a slot once, when the tape is loaded; natively each call reads the file afresh. A
  load that ran twice would find on CARDIAC what the program left in the slot, and fresh values natively: two
  answers. Making the array read-only was considered and rejected: once loaded it is memory like any other, and a
  program that reuses its cells (as scratch, or with `overlay()`) must be able to. Allowing two loads of a
  resource, each a slot of its own, was rejected too: a host binds one slot per resource for every run, and a tape
  whose code differs with the number of loads is not the one tape many data blocks are bound into.
- **Cost.** A program that wants a resource in a helper or a loop loads it in `main` first and passes it on. Every
  program that loaded a resource already did so: the five captured cases compile as before.

## 2026-10-19 · A loader binds resources, not a linker

- **Decision.** `carried::<N>()` is gone; a program loads a named resource, `let key: [i16; 10] =
  load_resource("key");`, the file `resources/key` beside the package's `Cargo.toml`, written as the input is. The
  name is required, a string literal. The tape records the slot as `# resource NAME ADDRESS LENGTH`, with `bound`
  after it once the slot's cards hold the values. `cargo cardiac build` binds each slot whose file is there, so the
  tape it writes is complete; `cargo cardiac run FILE.tape` binds a slot still empty from `resources/` beside the
  tape, and `--resource NAME=FILE` binds one from any file; a host binds in memory with `Tape::bind`. A slot still
  empty is an error that names it, and nothing is ever taken from the input for it. Natively, `load_resource` reads
  the file when it is called, from `CARGO_MANIFEST_DIR`, which `cargo run` sets, else from the current directory.
  In the listing the slot's label is `resource_NAME`. This supersedes in part "`carried()` is CARDIAC's memory
  mapping" and "`carried()`: tables that come on the tape" (2026-10-18).
- **Why.** Binding a slot is writing N values into N known cards: nothing is compiled, resolved or moved. That is a
  loader's job, done at load time, not a linker's. Data a host changes for every run is
  bound in memory just before it; a link step writing a new tape each time would add nothing. Natively the data is
  read when the program runs, never built in (no `include_bytes!`), so both builds read one file and it can change
  without a rebuild. A name, not "the first N values of the input", means the input is the program's own again, and
  the old rule that the table must come before the first `inp()` goes. The label is named after the resource, not
  after the `let` binding (`main_key`), so the listing shows what fills it whatever the variable is called.
- **Cost.** A program loads each resource once (twice is an error: pass the array on). `cargo cardiac test` runs a
  listing without a loader, so a resource's cells hold 0 there, as a test program's stub returns natively. Five
  captured cases call the old `carried::<N>()` and no longer compile; written with `load_resource`, they compile to
  the same cells, with only the label and the tape's line renamed. The flipboard runs bound tapes only.

## 2026-10-19 · Settings come from cargo profiles

- **Decision.** `cargo cardiac build` and `run` build with a Cargo profile, as `cargo build` does: `dev`, `release`
  with `--release`, or the one `--profile NAME` names. The MIR comes from `cargo check` on the package, with
  `--cfg cardiac --emit=mir` added to rustc's flags and a target directory of its own (`target/cardiac-bell-none`),
  so cargo resolves the profile itself: its defaults, `[profile.*]` in the workspace's `Cargo.toml`, `inherits`,
  per-package overrides, and each dependency's own settings. The profile's `overflow-checks` replaces the
  `// cardiac: overflow-checks=on` line, and its `opt-level`, which cargo reports for each program, also sets the
  compiler's own passes: off at 0, as they are today at 1 and 2, and the same as at 2 for 3, `s` and `z` for now.
  A single file has no `Cargo.toml`: it takes rustc's flags, `-C opt-level` and `-C overflow-checks`, with rustc's
  defaults. `MIR_OPT`, `OVERFLOW` and `--no-opt` are gone. This supersedes "Programs compile like a release build"
  (2026-10-16).
- **Why.** The goal is an idiomatic Rust tool, and a Rust programmer sets these in `Cargo.toml`. Asking cargo rather
  than reading the TOML ourselves means every way cargo has of setting a profile works, with no `toml` crate, and a
  program is checked as the binary it is, as a native build compiles it.
- **Cost.** rustc's MIR depends on more than the opt-level: debuginfo, debug assertions and incremental builds each
  change it (less is inlined). So the examples' `dev` profile, in the top-level `Cargo.toml`, sets all five for the
  `examples` and `cardiac` packages; the tool's own dev build is untouched. A profile is per package, not per
  program: `examples/collatz.rs` had its checks on by the comment, and the other examples need them off (`powers`
  doesn't fit with them), so collatz is now a package of its own, `examples/collatz/`, with checks on in `dev` and
  `release`, chosen with `-p collatz` as cargo chooses a package; its tape is the same as before. The two-crate
  test programs were `#![no_std]` binaries with no panic handler, which cargo refuses to check; they lost the
  attribute, and their MIR is the same. MIR at opt-level 0, cargo's `dev` default, parses for every test program,
  but with the passes off few fit in memory, and an iterator's `next` stays a call the
  compiler can't lower: an error there says so and points to the opt-level.

## 2026-10-19 · Patterns are tested on the IR, not its printed text

- **Decision.** A pass recognises a construct by its typed parts, never by how it prints:
  - a path by its segments (`Path::is_path(&["core", "intrinsics", "cold_path"])`);
  - a one-word constant by `Const::is_scalar`;
  - a type by its variant;
  - a symbol in the machine IR only where it is defined or named as an operand. A number never names one.
- **Why.** The tests were carried over from the compiler's regex days: "no space in the text", "no `)` in the type",
  "the text contains `i16::`". Each was an approximation that a new shape of rustc's output could slip past.
- **Cost.** None in output: no captured case changes. A program that hits an unsupported shape may get a different
  error message than before.

## 2026-10-19 · Range bounds are exact integers

- **Decision.** The range analysis keeps each bound as an `i64`, not as an `f64` carried over from the compiler's
  JavaScript days, when NaN stood for "this type has no range". Sums, differences and products saturate at
  `i64`'s ends.
- **Why.** An `f64` is inexact past 2^53, and a product of products outgrows any fixed width. A saturated bound
  is still wider than any value a cell can hold, so every decision it supports stays sound. `i32` is too narrow:
  the open side of a comparison is ±1e9.
- **Cost.** None in output: no bound in the captured cases passes about 2.2 million, where `f64` was exact.

## 2026-10-19 · The compiler reads MIR into a typed IR

- **Decision.** The compiler parses the text of `rustc --emit=mir` into a typed IR (`src/compile/ir`): a lexer, a
  recursive-descent parser, and the types rustc's own MIR has, named as rustc names them (`Place`, `Operand`,
  `Rvalue`, `Statement`, `Terminator`, `Ty`). The passes are typed transformations over it; the front end's are
  written as a mutable visitor (`ir/visit_mut.rs`, in the manner of rustc's `MutVisitor`). Lowering emits a typed
  machine IR (`src/compile/machine.rs`), the listing as lines of instructions, labels and data cells, which the
  runtime, the peephole rules, the phase splitter and the tape work on, and which is printed as assembly at the
  end. `src/compile/regex.rs` and the `fancy-regex` dependency are gone. This supersedes "The compiler is Rust: a
  straight port" (2026-10-18).
- **Why.** A pattern over text cannot tell `_1` from `_12` unless every pattern takes care to, and every pass had
  to parse again what the one before had printed. Typed values say what each line is, a pass matches on them, and
  a line it doesn't understand is an error rather than a missed match. A statement or terminator outside what the
  compiler can compile is kept as `Unsupported`, with rustc's text, so it is rejected with the line rustc wrote.
- **How it was checked.** One step at a time, each against the 466 captured cases: every case compiles to the
  same bytes. The changes in output made on purpose have entries of their own (the listing's layout, the order of
  a function's data cells).
- **Cost.** It still reads MIR as text, whose format rustc does not keep stable, so the compiler needs the rustc
  it was written for (1.97). Outside the captured cases, some error messages and some programs that could not
  compile before come out differently: `i32` becomes `i16` only in types, suffixes and paths, no longer in a name
  such as `to_i32`. Reading rustc's own data structures instead still needs nightly rustc and `rustc-dev`.

## 2026-10-19 · A fixed point compares facts, not their order

- **Decision.** The analyses run until nothing changes, repeated work and the range pass, decide that by
  comparing what they know as maps, whatever order the facts came in: repeated work keeps what is held in a
  `BTreeMap`, the range pass its facts and edges in `HashMap`s.
- **Why.** The facts were kept in `Ordered`, which compares the order too: the same facts in another order
  counted as a change and cost another round, and could run an analysis into its round limit (50 for repeated
  work; 40 for ranges, which is an error) after its facts had stopped changing. No later step reads their order:
  each fact is worked out on its own.
- **Cost.** None in cells: no captured case changes. Each round's facts are the same as before, so the widening
  after six rounds sees the same ranges; only the round that finds nothing changed can come sooner.

## 2026-10-19 · `carried()` is CARDIAC's memory mapping

Superseded in part by "A loader binds resources, not a linker" (2026-10-19): `carried()` is now
`load_resource("NAME")`, its slot `# resource NAME ADDRESS LENGTH`, bound from a named file, never from the input.

- **Decision.** `carried()` stays (see "tables that come on the tape", 2026-10-18). It is the closest CARDIAC
  comes to memory-mapped data: cells whose contents are put there before the program starts, by whoever builds
  the tape, and that the program only reads.
- **How it fits the machine.** CARDIAC has no way to put a value in memory except through its input: "Programs
  have to be loaded through the input, just like data" (the 1968 manual). The bootstrap, `002` and `800` with the
  `001` wired into cell 00, reads address and value pairs until the jump to the program's start. A table is more
  of those pairs: the loader puts it in memory with the program's own cells, and no instruction of the program
  reads it in. A host that builds a fresh tape for each run, with that run's table in it, is using the machine as
  intended; the tape's `# carried ADDRESS LENGTH` line tells it where the table goes.
- **Alternatives.**
  - Read the table with `INP` once the program starts: a second loading loop, doing the loader's job again, in
    the program's own cells. `examples/cipher.rs` is 57 cells with `carried()` and 70 with the loop.
  - A `const` array: also loaded with the program at no cost in code, but its values are fixed when the program is
    built. `carried()` is for values that change from run to run.
- **Cost.** It is not in the manual: on the paper machine, the operator writes the table's pairs on input cards
  after the bootstrap and the program, ahead of the jump to its start. `# carried` is a line of cargo-cardiac's
  `.tape` format, not of CARDIAC. Natively, `carried()` must come before the first `inp()`.
- **See also.** README.md, "Tapes are code, resources and hand-offs".

## 2026-10-19 · A function's data cells in the order of their locals

- **Decision.** A function's one-cell locals and fields are laid out by local number, each whole local before
  its fields, then the cells lowering makes up (a divide's remainder). The order is the symbols' own (`Sym` is
  `Ord`), not their names' text.
- **Why.** The cells were sorted by name as text, a leftover of the text compiler: `main_10` came before
  `main_2`, and `main_10_0` before `main_1_0` but after `main_1`. Sorting the names meant printing every symbol
  to compare it.
- **Cost.** None in cells. 227 of the 466 captured cases change, all in functions with a local numbered 10 or
  more: the same data lines in another order, so the cells after them have other addresses and the tapes differ.
  No program grows or stops fitting.

## 2026-10-19 · Comments carry no meaning

- **Decision.** No pass reads a comment in the machine IR. A `carried()` table's label is a line of its own kind
  (`Kind::Table`), and the tape finds the tables by it. To the peephole rules every label is a code label, with a
  comment or without; a table's label is data, as a labelled cell is, so it is neither a jump target nor a barrier.
- **Why.** The tape looked for a label whose comment started with `carried`, and the peephole rules took any
  commented label for a non-label, so editing a comment could change the program. Counting table labels as code
  labels instead breaks them: rule 6 drops a label no instruction names, and the 5 captured cases with tables lose
  their `# carried` line and the table's cards.
- **Cost.** None in cells: no captured case changes. The only commented labels the compiler emits are table
  labels, which the rules already passed over. A table label read back from assembly text is a plain label.

## 2026-10-19 · An address constant is a use of its cell

- **Decision.** Peephole rule 11 (drop a `STO` whose cell is read only by the next instruction) counts labelled
  `DAT` cells that name the cell, as rule 7 does: address constants (`a_y: DAT y`) and the instruction constants
  built from them. The store is still dropped when the code after it runs straight to `HRS` without a label, a jump
  or a use of the constant, since nothing can read the cell after that.
- **Why.** Rule 11 missed address constants, so it dropped the store into a local whose address was taken, and a
  read through the address got 0. Blocking the rule whenever a constant exists also fixed it, but cost
  2 cells at each spot and pushed two captured cases past cell 98; the straight-to-`HRS` case covers all of them.
- **Cost.** None in cells: no captured case changes.

## 2026-10-19 · The listing's layout belongs to the printer

- **Decision.** A line of the machine IR holds no spacing. The printer places every comment's `;` at one column
  (or one space past a longer line) and every labelled cell's `DAT` at another. The parser accepts any spacing.
- **Why.** The listing used to be text, and a rewritten instruction kept the spaces of the one it replaced, so
  comments drifted. Moving it to typed lines meant storing those spaces to keep the output byte-identical, which
  put layout in the IR, where every pass that rewrites a line had to carry it.
- **Cost.** A one-time whitespace change to the `.asm` listings: 373 of the 466 captured cases, none different
  once runs of spaces are collapsed. The tapes are unchanged.

## 2026-10-18 · The emulator's instructions are an enum, decoded by a match

- **Decision.** The ten instructions are `enum Op`, so the emulator's `match` covers them all with no catch-all
  arm. A cell's first digit becomes an `Op` through a `match` on the digit (`cardiac::op`).
- **Why not a table.** Indexing a `[Op; 10]` by the digit is the obvious way to write it, and it made the emulator
  25 to 40% slower: 300 million steps of a `CLA ADD TAC STO JMP` loop took 980–1160 ms against 750–810 ms for
  the `match`. Decoding runs on every write to a cell (every `STO`, `INP`, and the return jump `JMP` leaves in
  cell 99), so it sits in the hot loop. With the `match`, the enum runs as fast as the integer constants it
  replaced.
- **Cost.** None measured.

## 2026-10-18 · A program is an ordinary Cargo program

- **Decision.** A program is a Cargo binary with `fn main()`, and its built-ins come from a crate, `cardiac`, as a
  normal dependency (`use cardiac::*;`). `cargo run` runs it natively; `cargo cardiac build` compiles the same
  source for CARDIAC. Nothing in the source is there only for CARDIAC: no `#![no_std]`, no `include!`.
- **How the two builds differ.** rustc is given `--cfg cardiac` when it compiles for CARDIAC. The `cardiac` crate
  then has bodiless stubs, marked `#[inline(never)]` so rustc leaves the calls in the MIR, where the compiler turns
  them into instructions. Without the cfg, the built-ins have ordinary bodies on standard input and output.
- **How the compiler knows them.** By name, at the root of a crate: `inp`, `out`, `overlay`, `carried`, `digit`
  (`compile::BUILTINS`). A call to `cardiac::inp` is matched to the `cardiac` crate's `inp` like any call into
  another crate, and from then on it is the built-in. A program that defines its own `inp` at its root (the test
  programs do) gets the same built-in, so no crate name is written into the compiler.
- **`std`'s paths.** A program with `std` names what it takes from `core` by `std`'s paths in the MIR
  (`std::ops::Range`). They are rewritten to `core`'s before the passes run, since the passes look for those.
- **Why a crate rather than a file to include.** A dependency is how a Rust program already gets functions it
  didn't write, and it lets the native build be the program's own `cargo run`, with nothing to set up.
- **Cost.** The program is compiled as a library for its MIR, so rustc would warn that `main` is never used; that
  lint is turned off for that build.

## 2026-10-18 · Natively, the built-ins are standard input and output

- **Decision.** `inp()` reads the next number from standard input (numbers separated by spaces or new lines);
  `out()` prints one on a line of its own.
- **End of input.** The program stops quietly, as CARDIAC stops when its reader is empty, and prints what it
  printed until then. A word that isn't a number from -999 to 999 stops it with an error, since no card holds it.
- **Overflow.** `out()` of a value outside -999..=999 panics, and every native panic prints -999 first (a panic
  hook, set at the first call to a built-in), the card CARDIAC's panic routine punches. On CARDIAC the value can
  never reach the punch: with overflow checks on, the arithmetic that made it panics with -999; with them off, a cell
  keeps the low three digits. So the outputs agree when every value fits, and when an overflowing value goes
  straight to `out()` with checks on. `examples/collatz.rs`, which turns checks on, prints the same 37 cards both
  ways from 27, ending in -999.
- **Why not wrap natively, as a cell does.** Wrapping at `out()` would agree with CARDIAC only for a value printed
  as soon as it is made, and would print a wrong number without saying so. A panic says so.
- **Cost.** Values held natively are not checked until they are printed: native Rust keeps an `i16` past 999 that
  a cell would lose digits of. A panic before the first call to a built-in prints no -999.

## 2026-10-18 · The tape is the output; `cargo cardiac run` runs it

- **Decision.** `cargo cardiac build` writes, beside each program's assembly, `NAME.tape`: the cards CARDIAC loads
  it from, one per line. `cargo cardiac run` builds a program and runs its tape on the emulator, printing a card a
  line, so its output and `cargo run`'s can be compared as they are. The flipboard loads the same file.
- **Format.** Plain text, one signed number a line, `#` comments. Two comments carry meaning: `# phase N` before a
  later phase's tape, and `# carried ADDRESS LENGTH` for a table. A file anyone can read, edit and print.
- **Cells that start at 0** are left off the first phase's tape, since memory starts cleared: `examples/primes.rs`'
  tape has no card for any of its 50 sieve cells. A later phase loads over the one before, so all of its cells are
  punched.
- **Phases and input.** Each phase ends with JMP 00, the loader, and the next phase's tape goes in the reader then,
  ahead of the input still unread. The input is then one stream, read in order by whichever phase reads it, which is
  what the native build does. The alternative, every phase's tape first and the input after, works only for a
  program whose first phases read nothing. `examples/powers.rs` reads its input in its first phase. The emulator
  does this when told the phases (`cardiac::run_phases`); a JMP 00 from cell 02 is the loader's own and doesn't
  count.
- **Tables.** A `carried()` table's cells are on the first phase's tape, and `cargo cardiac run` and the flipboard
  fill them from the start of the input; natively `carried()` reads the same numbers. One stream again.
- **Size check.** A program or phase over 96 cells is now an error at build time, with its size, rather than a tape
  that runs past cell 98.
- **Names.** Output files are named after the binary target, not the source file, so `src/main.rs` of package
  `countdown` gives `countdown.tape`.

## 2026-10-18 · One command: `cargo cardiac`

- **Decision.** The package is `cargo-cardiac`, and its one binary is the cargo subcommand: `cargo cardiac build`,
  `run`, `test` and `cardiasm`. The tool's name is written in one place (`TOOL` in `src/lib.rs`), with the cfg it
  gives rustc (`compile::program::CFG`).
- **Why.** A program is a Cargo package, and `cargo install cargo-cardiac` then `cargo cardiac build` is what a
  Rust programmer expects. One binary is one name to keep from clashing with others.
- **Run in the wrong package.** Run in a package whose binaries can't be CARDIAC programs (cargo-cardiac's own, or
  one that needs a proc-macro crate), it stops and says to run it where the programs are, such as `examples/`.

## 2026-10-18 · Arrays of `bool`

- **Decision.** `[bool; N]` is an array of cells holding 0 or 1, like `[u8; N]`.
- **Why.** A sieve reads best as an array of crossed-out flags (`examples/primes.rs`). A `bool` local was already
  one cell; only arrays of them were turned away.
- **Cost.** None: arrays of other types compile as before.

## 2026-10-18 · A parameter passed straight on shares the argument's cell

- **Decision.** When a function kept as a call reads a parameter only once, as an argument of another call or as
  an operand of the multiply or divide routine, the parameter lives in that argument's cell. The caller stores
  straight into it, and the parameter's own cell and the copy (two instructions per call) go. It carries through
  more than one call: in `fn area(w, h) -> i16 { multiply(w, h) }` with `multiply` doing `x * y`, the caller of
  `area` stores into `rt_mul_a` and `rt_mul_b`.
- **When.** Only where the cell is sure to hold until the read: the read is in the function's first block, which
  nothing jumps back to; nothing before it in that block multiplies or divides (that would use the routines'
  cells); and the parameter is not part of a struct result, which the caller reads after the call. Calls only end
  a block, so no other call can come first. A parameter read twice, or changed, keeps its cell.
- **Why the size check.** The program is compiled with and without it, and the smaller one kept (with it on a
  tie). It should never be bigger, and the check makes sure. A program where no parameter qualifies is compiled
  once.
- **Cost.** None found: every other test program compiles to the same cells. `tests/programs/forward.rs` goes from
  114 cells, which doesn't fit, to 96, and runs 12 fewer instructions per pass of its loop. The routines change
  their own cells (the multiply negates its factors), which is one more reason the parameter must not be read
  again.

## 2026-10-18 · A phase is the blocks its start reaches

- **Decision.** Before main is lowered, its blocks are put in phase order: a phase's blocks are those its start
  reaches without passing an `overlay()`, kept in their own order, with the `overlay()` last. A block two phases
  reach is copied into the later one under a new name. The split after lowering is unchanged.
- **Why.** The split took main's code in rustc's block order and cut it at each `overlay()`. rustc puts an inlined
  function's blocks after the rest of the function. `tests/programs/phases.rs` fills an array of structs in its
  first phase through an inlined helper that prints as it goes (`points[n] = step(n as i16)`); the helper's blocks,
  which belong to the first phase's loop, came after the `overlay()`, so phase 1 jumped into phase 2.
- **Why copy rather than stop.** Each phase is loaded over the last, so a block both need has to be on both tapes.
  Functions and routines are already copied into every phase that calls them; a shared block of main is the same.
- **Cost.** None for programs whose blocks were already in phase order, which compile byte for byte as before; a
  copied block costs its cells again in the later phase. `cargo cardiac test` runs a phased program as tapes, one
  per phase, so the split is tested: `tests/programs/phases.rs`, and `phases2.rs`, where both phases need the same
  loop.

## 2026-10-18 · Programs use other crates, through Cargo

- **Decision.** A program can call functions and use consts from other crates. Programs are the binary targets of a
  Cargo package and name their crates as its dependencies; `cargo cardiac build` builds them. It reads the package
  with `cargo metadata`, has rustc compile each dependency to MIR and metadata with the program's own settings
  (overflow checks, opt-level), compiles the program against that metadata, and joins the MIR texts before the
  compiler reads them. From then on another crate's function or const is compiled as if it were the program's own,
  and only what `main` reaches goes on the tape. `cargo cardiac build FILE.rs` stays, for single files that use no
  other crate.
- **Why stable rustc's MIR, again.** A driver inside rustc would see every crate's MIR directly, but it needs nightly
  and `rustc-dev`. rustc already writes a crate's MIR with `--emit=mir`; it only writes it for one crate at a time.
  Compiling each crate in turn and joining the texts gets the whole program on stable.
- **Why Cargo.** It is how a Rust program already says what it depends on. A flag of the compiler's own for each
  crate was tried first and dropped: it worked the same inside, but it was one more thing to learn and keep in step
  with a `Cargo.toml` that said the same.
- **Why the same settings.** At opt-level 2 rustc inlines small functions across crates, taking their MIR from the
  dependency's metadata, so that MIR has to be built the way the program is. With overflow checks on in a program,
  the inlined code is checked too, as it was when the code was a module of the program.
- **Names.** rustc prints each name as short as it can while keeping it unique, and decides that separately in
  each crate: `twice` in its own crate's MIR, `kit::arith::twice` at a call in the program's. A call into another
  crate is matched to a function there by the end of its path. A dependency's function keeps the name its own crate
  prints, unless another crate prints the same name, when it gets its crate's name in front. With that, a program
  compiles to the same cells whether the code it uses is a module of it or a crate of its own.
- **Not covered.** A generic function rustc doesn't inline: its MIR exists only in generic form, so the build stops
  with an error naming the call. Two crates defining a const of the same name: consts are found by their last name,
  so the build stops and names it. Proc-macro crates, and features beyond the defaults.
- **Cost.** Each dependency is compiled once per setting per build, a fraction of a second. `cargo cardiac` reads
  `cargo metadata`'s JSON with `serde_json`.
- **Evidence.** `tests/programs/crates`: a call (kept and inlined), a const table read by the program and by the
  other crate, struct consts and struct results, and a function that calls another, with a function of the same
  name in the program. Each matches native Rust at both opt-levels, and each compiles to the same cells as with the
  other crate as a module. A generic call is rejected with its name.

## 2026-10-18 · The compiler writes CARDIAC assembly, in .asm files

- **Decision.** The compiler's output is called what it is, CARDIAC assembly, and written as `name.asm`. Its
  header says `cargo cardiac build`. The `.cardiasm` name and the word CARDIASM mean only the separate CARDIASM
  language (`cargo cardiac cardiasm`), which the compiler doesn't use.
- **Why not compile to CARDIASM.** The compiler already decides where every value lives, shares cells between
  values never needed together, and drops what is never read. CARDIASM would place variables itself, from 98
  down, and add nothing the compiler needs.
- **Cost.** Tape headers changed, so tapes are no longer byte-identical to the JavaScript compiler's, and the check
  against it is retired.

## 2026-10-18 · The compiler is Rust: a straight port

Superseded by "The compiler reads MIR into a typed IR" (2026-10-19): the passes no longer use regular expressions.

- **Decision.** The compiler, first written in JavaScript, is ported to Rust pass for pass, as `src/compile`. The
  port changes nothing the compiler does: every pass, in the same order, with the same regular expressions on the
  MIR text, run by the `fancy-regex` crate.
- **Why byte-identical.** A port that also improved things would leave no way to tell a porting mistake from a
  change. With the output identical for every test program at MIR opt-levels 0, 1 and 2, with overflow checks on
  and off, optimized and not, the port is checked pass by pass, and any later change to the compiler can be judged
  on its own.
- **Why the patterns stayed.** Rewritten as hand-made parsing, 2000 lines of regex-driven passes would each have
  needed checking again. Kept as they were, each Rust pass reads against the JavaScript line by line. They run
  on `fancy-regex`, not `regex`, because they use lookahead, lookbehind and back references. A regex engine
  written for the port was tried first and dropped: 560 lines nobody should have to trust, against one
  dependency, with identical output.
- **Cost.** The patterns are as hard to read as they were in JavaScript, and the MIR text format is just as
  unstable. Compiling is about twice as fast as node; a build of many programs is mostly rustc.
- **Next.** A driver that reads MIR as rustc's own data structures instead of its text. That needs nightly rustc
  with the `rustc-dev` component (`rustc_driver`, `rustc_middle`); this port runs on stable and needs neither.

## 2026-10-18 · Arrays of structs are one array per field; struct consts; results copied once

- **Decision.** An array of structs, `[Point; 3]`, is kept as one array per field: `points[i].y` is
  `points_y[i]`. Taking an element whole (`print(pts[i])`, `pts[1] = diagonal(7)` in `tests/programs/structs3.rs`) works field by field.
- **Why this layout.** Every array path the compiler already has (built instructions for a variable index, a
  constant index as a fixed cell, cards read or punched straight into an element) works on each field unchanged.
  Reading field y of element i needs only i, never i × 3, which CARDIAC would have to add up. Storing a struct's
  fields side by side would let a whole element be found from one address, but nothing here needs that.
- **Also:**
  - A struct const (`const CORNER: Point = ...`, or `Point::ORIGIN` in an `impl`) is replaced at each use by the
    struct it stands for.
  - A struct result is read straight out of the function's own cells: one copy, or none when it's only read once
    (`let p = diagonal(9); out(p.z)` punches the callee's cell). In the rewritten MIR the callee's cell is `@callee.N`.
  - A field read, through an index or not, goes straight into `out()` or a call's arguments without a scratch
    cell. A card read and later stored into an element is read straight into it.
- **Cost.** Each field array needs its own address constants for a variable index (a read and a write
  constant, a cell each, for each field read or written that way: `tests/programs/structs3.rs` has four), where
  one flat array with a computed offset would share one set. Programs without arrays of structs compile as before.
  Structs behind references are still not supported.
- **Evidence.** `tests/programs/structs3.rs` (arrays of structs) and `structs4.rs` (struct consts and results)
  match native Rust at both opt-levels.

## 2026-10-18 · Work out a repeated value once

- **Decision.** When a sum, difference, product, quotient, remainder or negation is worked out again from the
  same values, and every way into that point has already worked it out into one local with nothing changed since,
  the program takes it from that local instead.
- **Why.** At opt-level 2 an inlined helper called twice with the same argument is worked out twice: in
  `tests/programs/repeats.rs`, `far(a, b)` is `if size(a) < size(b) { size(b) } else { size(a) }`, and each
  `size` is worked out again in the branch it chooses. Writing it into a local by hand shouldn't be the only way to keep a program
  small.
- **What it took:**
  - It checks every way into a block, not just the one above it, so a value worked out on only one side of a
    branch isn't reused after the join.
  - A local whose address is taken never takes part.
  - Comparisons are left out: testing again is cheaper than keeping a bool.
  - A reuse is kept only when it makes the program smaller. A value that only ever sat in the accumulator would
    need a store and a cell of its own to keep, more than working out a sum again costs, so without this check a
    program could grow.
  - Where rustc has copied a test into two blocks, each setting and testing the same bool, the test and its branch
    still become one.
- **Cost.** Compile time only: a reuse is kept only when the program comes out smaller, so it can't make one
  bigger. Tests: `tests/programs/repeats.rs` and `repeats2.rs` (a sum worked out before a branch and again on one
  side of it, and again after one of its values changes, where it must be worked out afresh).

## 2026-10-18 · Expression form costs no more than statement form

- **Decision.** Three steps after the other clean-ups, so that `let h = hour(inp())`, `if v < 0 { -v } else { v }`
  and `let (a, b) = if c { (y, x) } else { (x + 1, y) }` compile as small as the same code written as statements:
  - **Sharing cells.** Two locals that are never needed at the same time, where one is made from the other, become
    one cell. The copy between them goes, and `h + 24` is done in place.
  - **Copies at a join.** Copies at the start of a block that both arms of an `if` jump to are tried in each arm
    instead, where they can be worked out with the arm's own lines. They stay there only when that leaves fewer
    lines.
  - **Block order.** A run of blocks starts at one that nothing else still to be placed jumps to, so more jumps
    become fall-throughs.
- **Why.** Readable Rust uses small helpers and `let` with a value. It shouldn't cost cells.
- **Limits.** Sharing leaves alone a local used in one block only (it already gets a shared scratch cell), and a
  function's result read once (it is read straight from the function's cell). Both made programs bigger.
- **Cost.** None known. The three forms are tested in `tests/programs/exprhelp.rs` (helpers that return a value),
  `exprif.rs` and `exprtuple.rs`.

## 2026-10-18 · Programs also build natively

- **Decision.** The built-ins have a second set of bodies, used in a native build, so the same source runs as an
  ordinary Rust program. (Since the built-ins became the `cardiac` crate, the native build is `cargo run`; see the
  first entry.) The CARDIAC build doesn't change: every tape compiles byte for byte the same.
- **Why.** It checks the compiler directly: the same source, compiled two ways, must print the same cards.
  `test_examples.sh` runs every example both ways, and `test_compiler.sh` every test program.
- **Choices:**
  - `overlay()` does nothing natively, so one call runs every phase.
  - `promise!()` is an `assert!` natively. On CARDIAC a broken promise is undefined; natively it is an error.
- **Cost.** Values aren't checked until they are printed: a value past 999 kept in a variable natively is caught
  only if it reaches `out()`.

## 2026-10-18 · `for` loops over ranges become counter loops

- **Decision.** `for i in a..b`, `a..=b` and either one `.rev()`, over `i16`, `u8` or `usize`, are turned back into
  the counting loop they stand for, before anything else runs (`compile::forloops`). rustc writes them as calls
  into the standard library: build a range, call `next()` until it returns nothing. The pass recognises those
  calls, and at opt-level 2 also rustc's own inlined version of them, and makes each one a test and a step on two
  cells, the range's start and end. Where the body doesn't need the counter's old value, the step moves to the end
  of the body and the counter itself is `i`.
- **Why.** A counted loop is the most common thing in a program, and `for i in 0..n` reads better than a `while`
  with its own counter.
- **Cost.** None in size: the loop compiles to the same code as the `while` it replaces, and with overflow checks
  on it is smaller, because the step can't overflow and needs no check. Programs rewritten with `for` came out the
  same size or smaller. An inclusive range can't run to the edge of a card (an end of 999, or a start of -999 when
  reversed): the counter would step one past it. Other iterators (`.iter()`, `.step_by()`, `zip`) still aren't
  supported.
- **Also fixed.** A checked add could store its result early into a block that was already compiled, which lost
  it; only blocks still to come are rewritten now. No existing program hit this.

## 2026-10-18 · Programs compile from MIR at opt-level 2

- **Decision.** rustc is asked for MIR at opt-level 2 (it was 1).
- **Why.** At opt-level 2 rustc inlines small functions in the MIR, so a helper (`read_point()`, `print(p)`)
  costs nothing on CARDIAC. That lets programs be written as readable Rust, with named helpers instead of repeated
  `inp()`/`out()` lines. Mark a function `#[inline(never)]` to keep one shared copy.
- **What it took:**
  - The inlined form of `promise!`, an `assume(b)` statement, now cuts its block and is treated like the
    un-inlined call.
  - An inlined `p.add(n)` on a raw pointer is `Offset(p, n)`, which is plain addition.
  - The range analysis now goes through the code in the order it runs, and only gives up on a growing range at
    the top of a loop. Inlining shuffles the order rustc writes the code in, and giving up anywhere else lost ranges
    it needed, and bounds checks came back.
- **Cost.** Mixed: inlining makes some programs smaller and some bigger (a helper called in several places is
  copied into each). `#[inline(never)]` keeps one copy where that is smaller. The test suite runs at both
  opt-levels.

## 2026-10-18 · Functions in modules keep their path

- **Decision.** `kit::arith::twice` becomes `kit__arith__twice`, at its definition and at every call.
- **Why.** Code is organised in modules; the old parser took only bare names.

## 2026-10-18 · Named consts by path; array consts become literals

- **Decision.** A `const` table (`pub const SQUARES: [i16; 16] = [...]`) is read from its body in the MIR, and
  every use of it by path (`const tables::SQUARES`) becomes the array literal. Scalar consts lose their path, and
  `i16` and `usize` consts now work as well as `u8`.
- **Why.** It lets a table live in a module, or in another crate, and still cost nothing extra: used at the top of
  `main`, an array literal becomes the cells' initial values on the tape, exactly as an inline one would
  (`tests/programs/tables.rs`, and `tests/programs/crates/xtable.rs` from the crate `kit`).
- **Cost.** Every use of a const is the whole literal, so a const table used in two functions takes its cells
  twice.

## 2026-10-18 · Structs become one cell per field

- **Decision.** A struct local is taken apart before any other pass runs (`mir::scalarize_structs`). Each field
  becomes a local of its own, named after the variable (`p_x`, `p_y`, `p_z` in the debug names). Building, copying
  and field access all turn into per-field statements.
- **Passing and returning.** Passing a struct passes its fields as separate arguments, so the callee's parameter
  splits the same way. A function returning a struct leaves the fields in its own cells, and the caller copies
  them out after the call (`@callee._N` in the rewritten MIR, the callee's cell `callee_N`).
- **Why.** CARDIAC has nothing to point at a record with except self-modifying code. Fields in separate cells
  cost nothing to read: `CLA p_x` is the same instruction as for any local. And every existing pass (the range
  analysis, copy forwarding, the scratch pool) already understands plain locals, so none of them had to learn
  about structs.
- **Costs.**
  - A struct returned by a function was copied twice: callee cells into a scratch cell, then into the
    destination (since fixed: see arrays of structs, above).
  - No struct behind a reference (`&Point`, `&mut Point`): the compiler stops with an error saying so. It needs a
    struct to have an address, which this layout deliberately avoids (since changed: a shared reference is the
    value, and a `&mut` is a `&mut` of each field, below).
  - Fields must be scalars, so there are no nested structs yet (since changed: see nested structs, below).
- **Evidence.** `tests/programs/structs.rs` and `structs2.rs` match native Rust. Programs without structs compile
  to byte-identical tapes.

## 2026-10-18 · `digit(v, k)` is built in, and compiles to a shift made on the spot

- **Decision.** Taking digit k of a three-digit card compiles to `SFT (k+1)3`, built for k and run (5 cells).
  The Rust function has a body, used by native builds, that says what it means.
- **Why.** When k is a variable, the Rust equivalent picks one of three divisions by it.
  `tests/programs/digit.rs`, which prints each digit of three numbers from an array, is 38 cells with `digit()`
  and 57 with `digit()`'s own Rust body written in its place; `examples/cipher.rs` is 57 cells with it and 71
  without.
- **Cost.** It is the one place where a hand-written CARDIAC trick lives in the compiler rather than coming
  from Rust code.

## 2026-10-18 · `promise!(c)` is `core::hint::assert_unchecked`

Superseded by "`assume()`, an unsafe fn, replaces `promise!`" (2026-10-20).

- **Decision.** No code. The condition becomes a fact for the range analysis, so bounds checks it proves can go.
- **Why.** Programs know things about their input that no analysis can see (a month is 1..=12). Rust already has
  the honest way to say that.
- **Cost.** A broken promise is undefined behaviour, as in Rust. Natively it is an `assert!`, so running the
  program with `cargo run` on the same input catches a wrong promise.

## 2026-10-18 · `carried()`: tables that come on the tape

Superseded in part by "A loader binds resources, not a linker" (2026-10-19): the table is a named resource, never
filled from the input.

- **Decision.** `let t: [i16; N] = carried();` compiles to nothing. It is a lookup table given at run time, ahead
  of the input: the tape records where its cells are (`# carried`), and its cells are filled from the start of the input when the tape is run, so CARDIAC's loader
  puts it in memory with the program's own cells. Natively `carried()` reads the same first N numbers.
- **Why.** The table is data, not code: a different key, price list or conversion table needs no new build. And
  loading it with the program costs no code. `examples/cipher.rs` takes its ten-digit key this way in 57 cells;
  read with `for i in 0..10 { key[i] = inp(); }` instead, it is 70.
- **Cost.** Natively, `carried()` must come before the first `inp()`, since its numbers are the first of the
  input.

## 2026-10-17 · Range analysis runs until nothing changes

- **Decision.** The analysis goes round the whole program until no range changes. A range still growing after six
  rounds jumps to the next of the program's own constants (a loop's limit, say), and six more rounds then pull the
  ranges back in where the loop's own test allows.
- **Why.** The first, single-pass version couldn't prove a loop counter in range. That cost bounds checks, and
  programs no longer fit.

## 2026-10-17 · The panic card

- **Decision.** Every failed check jumps to one shared routine, `rt_panic`, which punches -999 from a constant cell
  and halts (`OUT kn999`). One routine for every check keeps each check to a test and a jump.
- **Cost.** Three cells (the two instructions and the constant), in programs that keep a check.

## 2026-10-16 · Programs compile like a release build

- **Decision.** Overflow checks are off and bounds checks are on, as in `cargo build --release`.
  `// cardiac: overflow-checks=on` turns overflow checks back on.
- **Why.** Overflow checks cost cells on every add, and programs don't fit with them. A program whose values stay
  on a card never needs them; the native build, with checks on in a debug build, shows the ones that don't.

## 2026-10-12 · An `i16` is a card

- **Decision.** `i16` (and `i32`, Rust's default integer) lives in one cell, -999..999. Past that it panics
  when checks are on.
- **Why.** That's what a CARDIAC cell holds. The limit is CARDIAC's, not Rust's, and the README says so.

## 2026-10-12 · Take MIR from stable rustc; compile it ourselves

- **Decision.** rustc does parsing, types, the borrow checker and its own optimisation, and writes out MIR, its
  simplified form of the program (`--emit=mir`). cargo-cardiac compiles that text.
- **Why.** No nightly toolchain, no LLVM backend for a decimal machine, and every front-end guarantee Rust makes
  still holds.
- **Cost.** The MIR text format is unstable ("intended for human consumers only"), and only the crate being
  compiled has MIR (so other crates are compiled to MIR one by one: see "Programs use other crates"). A backend
  inside rustc is the long-term fix.
