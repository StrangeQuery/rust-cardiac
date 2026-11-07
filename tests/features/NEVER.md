# What cargo-cardiac will never compile

Most of what cargo-cardiac refuses is work not done yet. The rows on this page are different: they need something
CARDIAC doesn't have, so no amount of work in the compiler would make them right. Each one has a row here,
marked `never`, and the reason is the same for everything in its section.

CARDIAC has 100 cells, each holding a sign and three decimal digits, -999 to 999. Its accumulator holds four digits.
It reads cards, prints numbers, and has no other contact with the world. Everything below follows from that.

## Integers wider than 16 bits

`i64`, `u32`, `u64`, `i128` and `u128` are refused. A cell holds 1,999 values, and even a 16-bit integer holds far
more than that; cargo-cardiac accepts `i16`, `u8` and `usize` on the rule that a value must fit on a card and panics
when it doesn't. Wider types exist to hold values a card never could, so accepting them would only move the panic.

`i32` is the exception. It's the type Rust gives an integer literal when nothing else decides it, so programs use
it without meaning to, and cargo-cardiac accepts it under the same rule as `i16`. An `i32` past a card can't be held
even for a moment: `50 * 50 / 10` panics with overflow checks on, where Rust prints 250, and with them off it keeps
the last three digits of 2,500 and prints 50 (types/i32_past_card.rs).

`u16` and `isize` are not on this page: a `u16` or `isize` that fits on a card can be held as an `i16` is, so they
are left for when someone asks.

Rows: types/int_i64.rs, int_u32.rs, int_u64.rs, int_i128.rs, int_u128.rs, i32_past_card.rs.

## Byte order

`swap_bytes` on an `i16` moves its low byte to the high one, so almost every value on a card comes out far past
one: 100 becomes 25,600. A cell holds a decimal number, not two bytes, and the result has nowhere to go.

Row: integers/swap_bytes_i16.rs.

## Floating point

`f32` and `f64` are refused. CARDIAC has no fractions and no exponent, only whole numbers of three digits. A program
that needs fractions can keep them scaled: tenths in a cell are a number ten times as big.

Rows: types/float_f32.rs, float_f64.rs.

## The heap

`Box`, `Vec`, `String`, `vec!` and `format!` need an allocator, and a program's memory is the 96 cells it was
compiled into: every value has its cell, fixed when the program is built. A recursive enum (`enum List { Cons(i16,
Box<List>), Nil }`) needs the heap too, since nothing else can hold a list of unknown length.

Rows: macros/std_format_vec.rs, items/enum_recursive_box.rs.

## Text

`println!`, `format_args!` and `write!` format text, and CARDIAC prints only numbers: `out()` prints a value, and
that is the whole of its output. The formatting machinery is also dynamic dispatch through trait objects and digit
loops in `u32`, which a program in 96 cells couldn't hold anyway.

A string literal is a cell for each byte, so its length, its bytes as a table and `concat!` work, and cost what
they look like. Comparing strings or parsing them doesn't fit: a compare is a loop over every literal it might meet,
about 124 cells for two short ones, and a program's input is numbers on cards, so there is no text to parse.

Rows: macros/std_println.rs, format_args_write.rs, types/str_eq.rs,
integers/from_str_radix_i16.rs.

## Calls to the outside world

An `extern "C"` block declares a function defined somewhere else, and on CARDIAC there is nowhere else: the card
reader and the printer are the whole outside world.

Row: items/extern_block.rs.

## Reading one type's bits as another

A `union` read through the field it was written by works, and that row is supported. Reading it through another
field reinterprets one type's bits as another's, and a cell holds a decimal number, not bits, so there is nothing to
reinterpret.

Row: items/union_basic.rs (the supported case).

## Not ours to support

`let Some(x) = o;` is refused by rustc itself (E0005, a refutable pattern in a plain `let`), so it builds nowhere.
`let else` and `if let` are the ways round it, and both work.

Row: patterns/refutable_let.rs.
