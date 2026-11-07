//! Rust on CARDIAC.
//!
//! A program uses this crate for CARDIAC's card reader and teletype, `inp()` and `out()`, and for a few built-ins
//! that CARDIAC does its own way. The same program builds two ways:
//!
//! - `cargo run` builds it for your machine. `inp()` reads numbers from standard input, and `out()` prints them.
//! - `cargo cardiac run` builds it for CARDIAC with the rust-cardiac compiler and runs it on heartburn's `burn`.
//!   `inp()` is the instruction INP, and `out()` is OUT.
//!
//! ```no_run
//! use cardiac::*;
//!
//! fn main() {
//!     let n = inp();
//!     for i in (0..=n).rev() {
//!         out(i);
//!     }
//! }
//! ```
//!
//! Given 5, both builds print 5 down to 0.
//!
//! A CARDIAC cell holds a sign and three digits, so every value is from -999 to 999. A program whose values stay in
//! that range prints the same numbers both ways. With overflow checks on, arithmetic that leaves the range panics on
//! CARDIAC. With them off, a cell keeps the last three digits.
//!
//! To build for CARDIAC, install the cargo subcommands with `cargo install cargo-cardiac`, then
//! `cargo cardiac new countdown` makes a package like the one above. The guide, what Rust is supported and the
//! compiler's messages are at [lab.strangequery.com/rust-cardiac](https://lab.strangequery.com/rust-cardiac/).
//!
//! Each function's documentation says what it is on CARDIAC, what it costs there and what it does with `cargo run`.
#![cfg_attr(cardiac, no_std)]

// On CARDIAC. The compiler turns each call into instructions; these bodies are never run. `inline(never)` keeps
// rustc from inlining them into the program, so the calls stay where the compiler can see them.

/// On CARDIAC: INP. Documented on the native version.
#[cfg(cardiac)]
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// On CARDIAC: OUT. Documented on the native version.
#[cfg(cardiac)]
#[inline(never)]
pub fn out(_v: i16) {}

/// On CARDIAC: HRS n. Documented on the native version.
#[cfg(cardiac)]
#[inline(never)]
pub fn overlay() {}

/// On CARDIAC: an array the loader fills, at a label of its own. Documented on the native version.
#[cfg(cardiac)]
#[inline(never)]
pub fn load_resource<const N: usize>(_name: &str) -> [i16; N] {
    [0; N]
}

/// Digit `k` of a value 0..=999: `k` = 0 is the hundreds, 1 the tens, 2 the units.
///
/// On CARDIAC it is the shift that keeps that digit, `SFT (k+1)3`, built for `k` and run: 5 cells, where the
/// same thing in Rust arithmetic costs about 9. Natively it is this body.
#[inline(never)]
pub fn digit(v: i16, k: i16) -> i16 {
    if k == 0 {
        v / 100
    } else if k == 1 {
        v / 10 % 10
    } else {
        v % 10
    }
}

/// On CARDIAC: a jump into a row of `INP 00`. Documented on the native version.
#[cfg(cardiac)]
#[inline(never)]
pub fn skip_cards(_n: i16) {}

/// On CARDIAC: a jump into a row of `ADD c`. Documented on the native version.
#[cfg(cardiac)]
#[inline(never)]
pub fn add_times(_c: i16, _n: i16) -> i16 {
    0
}

/// A `bool` for a flag kept in a cell and tested more than once: the same trues and falses, and on CARDIAC one
/// instruction less per test.
///
/// ```no_run
/// use cardiac::*;
///
/// fn main() {
///     let mut seen_zero: Flag = false.into();
///     loop {
///         let v = inp();
///         if v == 0 {
///             seen_zero = true.into();
///         }
///         if seen_zero.get() {
///             out(v);
///         }
///     }
/// }
/// ```
///
/// CARDIAC has one test, TAC: jump if the accumulator is negative. A `bool` is 0 or 1, as Rust promises, so testing
/// one kept in a cell is three instructions (load it, take away 1, TAC). A `Flag` holds -1 for false and 0 for
/// true, so testing it is two (load it, TAC): a cell and a step less each time, at the cost of one cell holding
/// -1, which a program needs once. `cargo cardiac build` says when a program's bools would be smaller as `Flag`s.
///
/// A condition tested where it is worked out (`if a < b`) is never kept in a cell, and needs no `Flag`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Flag(i16);

impl Flag {
    /// `false`, for a `const` (where `false.into()` can't go).
    pub const FALSE: Flag = Flag(-1);
    /// `true`, for a `const`.
    pub const TRUE: Flag = Flag(0);

    /// Its value, for `if` (which takes only a `bool`).
    #[inline(always)]
    pub fn get(self) -> bool {
        self.0 >= 0
    }
}

impl From<bool> for Flag {
    #[inline(always)]
    fn from(b: bool) -> Flag {
        if b {
            Flag::TRUE
        } else {
            Flag::FALSE
        }
    }
}

impl From<Flag> for bool {
    #[inline(always)]
    fn from(f: Flag) -> bool {
        f.get()
    }
}

impl Default for Flag {
    /// `false`, as `bool`'s default is.
    #[inline(always)]
    fn default() -> Flag {
        Flag::FALSE
    }
}

impl core::ops::Not for Flag {
    type Output = Flag;

    /// -1 and 0 swap: -1 - f.
    #[inline(always)]
    fn not(self) -> Flag {
        Flag(-1 - self.0)
    }
}

/// On CARDIAC: `core::hint::assert_unchecked`. Documented on the native version.
///
/// # Safety
///
/// `condition` must hold.
#[cfg(cardiac)]
#[inline(always)]
pub unsafe fn assume(condition: bool) {
    if cfg!(debug_assertions) {
        assert!(condition);
    } else {
        // SAFETY: the caller's promise that condition holds
        unsafe { core::hint::assert_unchecked(condition) }
    }
}

// Natively: standard input and output.

#[cfg(not(cardiac))]
mod native {
    use std::collections::VecDeque;
    use std::io::{BufRead, Write};
    use std::path::PathBuf;
    use std::sync::{Mutex, Once};

    /// Numbers read from standard input and not yet taken.
    static INPUT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
    static PANIC_CARD: Once = Once::new();

    /// The loader runs `before_main` before `main`, as it runs a C++ global's constructor: the program's pointer to
    /// it is in the section of constructors, so a panic before the first `inp()` or `out()` prints -999 too.
    /// Elsewhere the first `inp()`, `out()` or `load_resource()` sets up.
    #[used]
    #[cfg_attr(target_vendor = "apple", link_section = "__DATA,__mod_init_func")]
    #[cfg_attr(all(unix, not(target_vendor = "apple")), link_section = ".init_array")]
    #[cfg_attr(windows, link_section = ".CRT$XCU")]
    static BEFORE_MAIN: extern "C" fn() = before_main;

    extern "C" fn before_main() {
        setup();
    }

    /// A panic prints -999 first, as CARDIAC's panic routine does (then Rust's own message, on standard error).
    pub fn setup() {
        PANIC_CARD.call_once(|| {
            let rust = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                println!("-999");
                let _ = std::io::stdout().flush();
                rust(info)
            }));
        });
    }

    fn stop(message: Option<String>) -> ! {
        let _ = std::io::stdout().flush();
        match message {
            None => std::process::exit(0),
            Some(m) => {
                eprintln!("{m}");
                std::process::exit(2)
            }
        }
    }

    /// A word of the input or of a resource as a card, or a stop that says what it is.
    fn card(word: &str, from: &str) -> i16 {
        match word.parse::<i16>() {
            Ok(v) if (-999..=999).contains(&v) => v,
            _ => stop(Some(format!(
                "{from}not a card: {word} (a card holds a number from -999 to 999)"
            ))),
        }
    }

    pub fn next() -> i16 {
        setup();
        let mut input = INPUT.lock().unwrap();
        while input.is_empty() {
            let mut line = String::new();
            match std::io::stdin().lock().read_line(&mut line) {
                // the reader is empty: CARDIAC stops, and so does the program
                Ok(0) => stop(None),
                Ok(_) => input.extend(line.split_whitespace().map(String::from)),
                Err(e) => stop(Some(format!("reading standard input: {e}"))),
            }
        }
        let word = input.pop_front().unwrap();
        card(&word, "")
    }

    pub fn resource<const N: usize>(name: &str) -> [i16; N] {
        setup();
        // beside the package's Cargo.toml when cargo runs the program; else where it is run from
        let dir = std::env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_default();
        let path = dir.join("resources").join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| stop(Some(format!("resource {name}: {}: {e}", path.display()))));
        let from = format!("resource {name}: {}: ", path.display());
        let values: Vec<i16> = text.split_whitespace().map(|w| card(w, &from)).collect();
        let n = values.len();
        values
            .try_into()
            .unwrap_or_else(|_| stop(Some(format!("{from}{n} values, and the program loads {N}"))))
    }
}

/// Reads the next card.
///
/// On CARDIAC this is INP, one instruction, which reads a card into memory. When the reader is empty, the machine
/// stops.
///
/// With `cargo run`, it reads the next number from standard input. Numbers are separated by spaces or new lines.
/// When there are none left, the program stops, as CARDIAC does. Anything that isn't a number from -999 to 999 stops
/// it with an error.
#[cfg(not(cardiac))]
pub fn inp() -> i16 {
    native::next()
}

/// Outputs a value.
///
/// On CARDIAC this is OUT, one instruction, which sends a cell to the teletype.
///
/// With `cargo run`, it prints `v` on a line of its own. A value outside -999..=999 can't be on a card, so it
/// panics. Every panic with `cargo run` prints -999 first, which is what CARDIAC outputs when it panics. On CARDIAC
/// such a value never reaches `out()`: with overflow checks on, the arithmetic that made it panics, and with them
/// off, the cell keeps the last three digits.
#[cfg(not(cardiac))]
pub fn out(v: i16) {
    native::setup();
    assert!(
        (-999..=999).contains(&v),
        "out({v}): a card holds a number from -999 to 999"
    );
    println!("{v}");
}

/// Ends a phase of a program too big for CARDIAC's memory.
///
/// On CARDIAC this is HRS n: the machine stops for deck n, the next phase. The loader reads that phase's code in
/// over this one's, and the data stays where it is. A program in phases builds as `NAME.cqb`, `NAME.2.cqb` and so
/// on. Put `overlay()` in `main` where each phase ends, or turn on `auto-overlay` under
/// `[package.metadata.cardiac]` and the compiler places the phases itself.
///
/// With `cargo run`, it does nothing. The whole program is in memory, so one phase runs straight into the next.
#[cfg(not(cardiac))]
pub fn overlay() {}

/// An array of `N` values from the file `resources/NAME`, beside the package's `Cargo.toml`.
///
/// ```no_run
/// # use cardiac::*;
/// let key: [i16; 10] = load_resource("key");
/// ```
///
/// The file holds numbers separated by spaces or new lines, as the input does.
///
/// On CARDIAC the loader puts the values in memory with the program, so the array costs no code. Call it once, in
/// `main` and outside any loop, since the loader fills it once. The program may change the array afterwards.
///
/// With `cargo run`, the file is read when the program runs, so it can change between runs. It is found through
/// `CARGO_MANIFEST_DIR`, or the current directory when the program is run some other way. A missing file, a value
/// that isn't a card, or a count other than `N` stops the program with an error.
#[cfg(not(cardiac))]
pub fn load_resource<const N: usize>(name: &str) -> [i16; N] {
    native::resource(name)
}

/// Reads `n` cards and drops them, as `for _ in 0..n { inp(); }` does. `n` is from 0 to 999.
///
/// On CARDIAC it jumps into a row of ten `INP 00`: once for the units of `n`, then through the whole row once for
/// each ten. A call takes 17 instructions, plus one for each card and two more for each ten, about 1.2 a card. The
/// loop takes 6 a card. The row is a routine put in once (27 cells), and each call costs 2 cells. A constant `n` of
/// 3 or less is that many `INP 00` in place. An `n` outside 0..=999 is undefined on CARDIAC.
///
/// With `cargo run`, it calls `inp()` `n` times. An `n` outside 0..=999 panics.
#[cfg(not(cardiac))]
#[track_caller]
pub fn skip_cards(n: i16) {
    assert!((0..=999).contains(&n), "skip_cards({n}): n is 0 to 999");
    for _ in 0..n {
        inp();
    }
}

/// `n * c`, as `c` added up `n` times. `n` is from 0 to 99, and the product must be from -999 to 999.
///
/// On CARDIAC it jumps into a row of nine `ADD c`: once for the tens digit of `n`, then a shift, then again for the
/// units. A call takes about 27 instructions plus one for each addition, so `add_times(c, 37)` takes 37, where a loop
/// at 7 instructions an addition takes 263. The row is a routine put in once (34 cells), and each call costs 4 cells. A
/// constant `n` becomes `c * n` in place, with adds and a shift. An `n` or a product out of range is undefined on
/// CARDIAC.
///
/// With `cargo run`, it multiplies. An `n` or a product out of range panics.
#[cfg(not(cardiac))]
#[track_caller]
pub fn add_times(c: i16, n: i16) -> i16 {
    native::setup();
    assert!((0..=99).contains(&n), "add_times({c}, {n}): n is 0 to 99");
    let product = c as i32 * n as i32;
    assert!(
        (-999..=999).contains(&product),
        "add_times({c}, {n}): {product} doesn't fit on a card"
    );
    product as i16
}

/// Tells the compiler something about a value that it can't see for itself, such as a month always being from 1
/// to 12, so it can leave out checks it would otherwise need.
///
/// On CARDIAC it is `core::hint::assert_unchecked` and costs no code. With debug assertions on
/// (`debug-assertions = true` in the profile), CARDIAC checks it too, and a false assumption panics as `assert!`
/// does.
///
/// With `cargo run`, it is always checked. A false assumption panics and names the line that made it.
///
/// # Safety
///
/// `condition` must hold. On CARDIAC without debug assertions, a false assumption is undefined behaviour: the
/// checks it let the compiler drop are gone, and the program reads or computes whatever is there.
#[cfg(not(cardiac))]
#[track_caller]
pub unsafe fn assume(condition: bool) {
    assert!(condition, "a broken assumption");
}
