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
