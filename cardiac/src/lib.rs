//! The built-ins of a Rust program for CARDIAC.
//!
//! ```no_run
//! use cardiac::*;
//!
//! fn main() {
//!     let n = inp();          // the next card from the reader
//!     for i in (0..=n).rev() {
//!         out(i);             // a value output, a line printed
//!     }
//! }
//! ```
//!
//! Built with `cargo run`, the program runs on your machine: `inp()` reads a number from standard input and
//! `out()` prints one. Built and run with `cargo cardiac run`, each call is one CARDIAC instruction: `inp()` is
//! INP, the card reader, and `out()` is OUT, the teletype. The two builds print the same numbers.
//!
//! A CARDIAC cell holds a sign and three digits, so every value a card carries is -999..=999.
#![cfg_attr(cardiac, no_std)]

// On CARDIAC. The compiler turns each call into instructions; these bodies are never run. `inline(never)` keeps
// rustc from inlining them into the program, so the calls stay where the compiler can see them.

/// The card reader: the next card. INP.
#[cfg(cardiac)]
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// The teletype. OUT.
#[cfg(cardiac)]
#[inline(never)]
pub fn out(_v: i16) {}

/// The end of a phase. HRS n: the machine stops for deck n, the next phase's, which the loader reads in over this
/// one's code (the data stays).
#[cfg(cardiac)]
#[inline(never)]
pub fn overlay() {}

/// A resource, `resources/NAME`, that the loader puts in memory with the program. No code: the array lives at a
/// label of its own, and the deck names its cells so they can be filled when it is built or loaded. Called once, in
/// `main` and on no loop, as the loader fills it once; the program may write to the array after.
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

/// Reads `n` cards and drops them: `for _ in 0..n { inp(); }`. `n` is 0..=999.
///
/// On CARDIAC it is a computed jump into a row of ten `INP 00`: in for the units, then the whole row once for each
/// ten. A call takes 17 instructions, one more a card and two more a ten (about 1.2 a card), where the loop takes 6
/// a card. The row is a routine put in once (27 cells), a call to it 2 cells; a constant `n` of 3 or less is that
/// many `INP 00` in place. Natively, an `n` outside 0..=999 panics; on CARDIAC it is undefined.
#[cfg(cardiac)]
#[inline(never)]
pub fn skip_cards(_n: i16) {}

/// `c` added up `n` times: `n * c`. `n` is 0..=99, and `n * c` must fit on a card (-999..=999).
///
/// On CARDIAC it is a computed jump into a row of nine `ADD c`: in for the tens digit of `n`, a shift, and in again
/// for the units. A call takes about 27 instructions and one more an addition, T for the tens and u for the units
/// (`add_times(c, 37)` is 37, a loop's 7 an addition 263). The row is a routine put in once (34 cells), a call to
/// it 4 cells; a constant `n` is `c * n` in place (adds and a shift). Natively, an `n` outside 0..=99 or a product
/// past a card panics; on CARDIAC they are undefined.
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

/// An assumption about a value that the compiler can't see for itself (a month is always 1..=12, say), so it can
/// leave out the checks it would otherwise need.
///
/// On CARDIAC it is `core::hint::assert_unchecked`: no code. With debug assertions on (`debug-assertions = true`
/// in the profile), it is checked on CARDIAC too: a false assumption panics, as `assert!` does. Natively it is
/// always checked.
///
/// # Safety
///
/// `condition` must hold. On CARDIAC without debug assertions a false assumption is undefined behaviour: the checks
/// it let the compiler drop are gone, and the program reads or computes whatever is there.
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

/// The card reader: the next number on standard input (numbers are separated by spaces or new lines). When there
/// are no more, the program stops, as CARDIAC stops when its reader is empty. Anything that isn't a number from
/// -999 to 999 stops it with an error.
#[cfg(not(cardiac))]
pub fn inp() -> i16 {
    native::next()
}

/// The teletype: prints `v` on a line of its own.
///
/// A value outside -999..=999 doesn't fit on a card, and panics. Like every panic here, it prints -999 first, the
/// value CARDIAC outputs when it panics. CARDIAC itself never gets that far: built with overflow checks on, the
/// arithmetic that made the value panics; built without (as in release), the cell keeps the low three digits.
#[cfg(not(cardiac))]
pub fn out(v: i16) {
    native::setup();
    assert!(
        (-999..=999).contains(&v),
        "out({v}): a card holds a number from -999 to 999"
    );
    println!("{v}");
}

/// Nothing: natively the whole program is in memory at once, so one phase simply runs on into the next.
#[cfg(not(cardiac))]
pub fn overlay() {}

/// A resource: the `N` values of the file `resources/NAME`, separated by spaces or new lines as the input is. On
/// CARDIAC the loader puts them in memory with the program. Natively the file is read here, when the program runs
/// (never built into it, so it can change between runs), from beside the package's `Cargo.toml`, the directory
/// `cargo run` gives as `CARGO_MANIFEST_DIR`; run some other way, from the current directory. A missing file, a
/// value that isn't a card, or other than `N` values stops the program with an error.
#[cfg(not(cardiac))]
pub fn load_resource<const N: usize>(name: &str) -> [i16; N] {
    native::resource(name)
}

/// Reads `n` cards and drops them. On CARDIAC a computed jump into a row of `INP 00`s; here, `n` calls of `inp()`.
/// An `n` outside 0..=999 panics.
#[cfg(not(cardiac))]
#[track_caller]
pub fn skip_cards(n: i16) {
    assert!((0..=999).contains(&n), "skip_cards({n}): n is 0 to 999");
    for _ in 0..n {
        inp();
    }
}

/// `n * c`, as `c` added up `n` times. On CARDIAC a computed jump into a row of `ADD c`s. An `n` outside 0..=99, or
/// a product that doesn't fit on a card (-999..=999), panics.
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

/// An assumption about a value that the compiler can't see for itself, so it can leave out the checks it would
/// otherwise need: on CARDIAC (without debug assertions), no code. Natively it is checked: a false assumption
/// panics, naming the line that made it.
///
/// # Safety
///
/// `condition` must hold: on CARDIAC without debug assertions a false assumption is undefined behaviour.
#[cfg(not(cardiac))]
#[track_caller]
pub unsafe fn assume(condition: bool) {
    assert!(condition, "a broken assumption");
}
