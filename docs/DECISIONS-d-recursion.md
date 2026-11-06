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
