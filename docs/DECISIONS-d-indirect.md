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
