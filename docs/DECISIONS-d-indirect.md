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
