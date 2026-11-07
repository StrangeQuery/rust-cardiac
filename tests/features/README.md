# The feature tests

What cargo-cardiac compiles and what it doesn't, one small program per feature, in the Rust Reference's order.
[SUPPORT.md](SUPPORT.md) is the result. Every status in it comes from running the programs, never from what
someone wrote down. [NEVER.md](NEVER.md) explains the rows marked `never`: what CARDIAC can't do, and why. The top
of [src/main.rs](src/main.rs) says how a row is written.

From the top of the repository:

    ./test_features.sh                # every row
    ./test_features.sh for_           # the rows whose path contains for_

It builds cargo-cardiac and its driver in release first, and tests those, or the cargo-cardiac that
`CARGO_CARDIAC` names.
