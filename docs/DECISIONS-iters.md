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
