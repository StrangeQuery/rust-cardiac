Two wrong answers in 0.1.3 are fixed. A value borrowed for a call could share a cell with another value and be read
wrongly, and an overflow-checked sum of three terms could lose its first term when another of them was an array
element at an index worked out at run time.

Much more Rust compiles: slices of run-time length and their methods, iterator adaptors (`map`, `filter`, `zip`,
`chain`, `skip`, `take` and the rest), function pointers and `dyn Trait`, closures passed, returned and stored,
recursion (a function that calls itself gets a stack of its own, and functions that call one another at their ends
need none), the bitwise operators and the bit methods, `u16` and `isize`, panic messages with values in them,
nested tuples and arrays of arrays, the integer methods (`pow`, `isqrt`, the saturating ones and the rest), and a
string literal's length and bytes.

Every message about a program now has a code: RCE for an error, RCW for a warning and RCI for a note. `cargo cardiac
help CODE` says what one means, and so does its page in
[the message codes](https://lab.strangequery.com/rust-cardiac/messages/). A program that uses an expensive
operation, such as `&` between two variables or `count_ones()`, gets a warning that names it. `--edition` sets a
single file's edition. And `cargo cardiac` says when a newer version is out, at most once a day
(`CARGO_CARDIAC_NO_UPDATE_CHECK=1` turns that off).

[docs/RUST.md](https://github.com/StrangeQuery/rust-cardiac/blob/HEAD/docs/RUST.md) has the whole list of what
compiles, what it costs, and what doesn't and why.
