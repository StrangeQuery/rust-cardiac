# Items

[The manual](../manual.md) › Items

## What's different on CARDIAC

- Recursion other than a call at the end of the function needs a stack of its own, about 30 cells for each place the
  function calls itself. Fibonacci's two calls don't fit in 96 cells.
- An inlined helper is copied into every caller, and each copy costs cells. `#[inline(never)]` keeps one.
- An array passed by value is copied at every call. A struct behind a `&mut` passes an address for each of its cells.
- An `extern "C"` block is refused: there is nothing on the other side of the call.

## Functions

Functions take parameters and return a result. Small helpers that rustc inlines cost nothing extra.
`#[inline(never)]` keeps one shared copy of a helper. Without it, an inlined helper called in many places is copied
into each one. Generic functions work, inlined or not, and so do const generics, `const fn`, `impl Trait` as a
parameter and as a result, and methods taking `self`, `&self` or `&mut self`.

**Recursion.** A function's call of itself as the last thing it does is a loop. Functions that call one another at
their ends are one function. Neither needs a stack.

Other recursion has a stack of its own. It costs about 30 cells of code for each place the function calls itself,
and a cell a call for each local still read after the call. The stack is as deep as the memory left over holds, and
a call with no room panics with `stack overflow`. Fibonacci's two calls don't fit in 96 cells. A loop is cheaper,
or a call of itself as the last thing the function does.

Not done yet:

- a recursive function with a reference or a pointer among its locals or parameters;
- one that returns a struct;
- one that keeps a local other than a number or a bool across the call;
- recursion through more than one function, unless every call is at its end.

Write a loop instead, or pass values in and return them.

**Passing values.** An array passed by value is copied element by element at every call: two instructions an
element, and the parameter's own cells. Pass `&[T; N]` instead. A struct passed or returned by value copies each
field, which is cheap for a few fields. A struct or an enum behind a `&mut` passes a `&mut` of each of its cells:
an address a cell, and each read or write through one is self-modifying code, a few cells and steps. For a function
that touches every field, pass the value in and return it.

## Structs

Structs with named fields, tuple structs, unit structs and generic structs work, with methods (`impl`), functional
update and destructuring. A unit struct needs no cells. For how a struct is laid out, and what an array of structs
costs, see [Types](types.md#structs).

## Enumerations

**C-like enums** (`enum Colour { Red, Green, Blue }`) are one cell holding the variant's number. The numbers can be
given (`Low = -5`, `High = 3 * 300`) or not, with `#[repr(i16)]` or `#[repr(u8)]` or neither. They work in
`match`, with `as i16`, passed and returned, and with `==`, `<` and `cmp` from a derived `PartialEq`, `PartialOrd`
or `Ord`.

**Enums with data** (`enum Shape { Dot, Circle(i16), Rect { w: i16, h: i16 } }`) work, as do generic enums of the
program's own, `Option` and `Result`. They can be built, matched, used in `if let`, passed, returned and nested. They
cost a tag cell: see [Types](types.md#enums).

## Unions

A `union` works when it is read through the field it was written by.

## Constant and static items

`const` values and tables work, also from other crates, and so do `static` and `static mut`. A constant has to fit
on a card: see [Constant evaluation](constant-evaluation.md).

## Traits and implementations

Traits work with static dispatch, default methods, associated types and constants, supertraits, generic traits and
blanket impls. Operator overloading works for `Add`, `AddAssign`, `Neg`, `PartialEq` and `PartialOrd`. For
`&dyn Trait`, see [Types](types.md#trait-objects).

## External blocks

An `extern "C"` block is refused: there is nothing on the other side of the call. An `extern "C" fn` defined in
Rust works. Write a Rust function.

Feature tests: [items](../../tests/features/items).
