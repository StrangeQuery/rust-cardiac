<p align="center"><img src="https://raw.githubusercontent.com/StrangeQuery/rust-cardiac/HEAD/assets/carbide.png" alt="CARBIDE" width="256"></p>

# cardiac

The built-ins of a Rust program for CARDIAC, for
[rust-cardiac](https://github.com/StrangeQuery/rust-cardiac).

<p align="center"><img src="https://raw.githubusercontent.com/StrangeQuery/rust-cardiac/HEAD/assets/cardiac.jpg" alt="A CARDIAC: the cardboard computer, its slides pulled out" width="360"></p>

```rust
use cardiac::*;

fn main() {
    for n in (0..=10).rev() {
        out(n);
    }
}
```

`cargo run` runs the program on your machine: `inp()` reads a number from standard input, `out()` prints one on
a line of its own. `cargo cardiac build` (from the `cargo-cardiac` package) compiles the same program for CARDIAC,
where `inp()` is the card reader and `out()` the teletype, and burn, from cardiaq, runs the deck it builds. The two
print the same numbers.

| built-in             | on CARDIAC                                       | natively                                       |
|----------------------|--------------------------------------------------|------------------------------------------------|
| `inp() -> i16`       | INP: the next card                               | the next number on standard input; none: stop  |
| `out(v: i16)`        | OUT: outputs a value                             | prints `v`; outside -999..=999 it panics        |
| `overlay()`          | ends a phase of a program bigger than memory     | nothing                                        |
| `load_resource("k")` | an array the loader fills from the resource `k`  | the file `resources/k`, read when it is called |
| `digit(v, k)`        | digit k of v (0 hundreds, 1 tens, 2 units), SFT  | the same, in arithmetic                        |
| `assume(cond)` (unsafe) | `core::hint::assert_unchecked`: no code; checked with debug assertions on | `assert!`              |
| `Flag`               | a flag as -1 (false) or 0 (true): tested by its sign | the same `i16`                              |

A panic prints -999 first, the value CARDIAC's panic routine outputs. See the rust-cardiac README for the rest.
