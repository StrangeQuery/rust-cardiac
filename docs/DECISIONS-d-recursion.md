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
