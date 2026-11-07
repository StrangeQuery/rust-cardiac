//! What cargo-cardiac says about a program, each message with a code, as rustc's errors have: `error[RCE0002]` for
//! an error or a refusal, `warning[RCW0001]` for a warning and `note[RCI0001]` for information. The message is one
//! line; what it means on CARDIAC, why, and what to write instead is the code's explanation, which `cargo cardiac
//! help CODE` prints and the website has a page of (`/rust-cardiac/messages/CODE/`). The table below is the only
//! place either is written: every message about a program is made by one of its constructors, and the website's
//! pages are made from `cargo cardiac help --json`. A usage error (a bad flag) and the tool's own failures (no
//! driver, a file it can't write) are no code's: `Diag::tool`, said as a plain `error:`.
//!
//! ## Adding a code
//!
//! - A code is what went wrong, not where: a new message that fits a code already here is another constructor in
//!   that code's entry (its explanation may need a sentence more). A new kind of problem is a new entry.
//! - A new entry takes the next number of its kind, at the end of that kind's entries: RCE for errors, RCW for
//!   warnings, RCI for notes. A code is never renumbered or reused, as people search for it; one no longer made
//!   stays, its explanation saying so.
//! - An entry is its code, its kind, a title (a few words, capitalized, for the index and the page's heading), its messages, and its explanation in
//!   Markdown: a sentence saying what it means, then why, what to write instead, and a small example in a
//!   ` ```rust ` block. The website reads paragraphs, lists, code blocks, `code` and **bold**, nothing more.
//! - Each message is `name(param: Type, ...) "text with {param}"`, one line in rustc's style: lowercase, no full
//!   stop, the operand or line it is about at the end. It makes a `Diag`: `return Err(codes::name(...))`.
//! - Then regenerate the website's copy: `cargo cardiac help --json > rust_cardiac_codes.json` in the site's
//!   repository. The tests below check the table's shape.
use std::fmt::{self, Display};

use serde_json::{json, Value};

/// What a message is: a refusal or an error, a warning, or information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Error,
    Warning,
    Note,
}

impl Kind {
    /// The word a message starts with, as rustc's do.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Error => "error",
            Kind::Warning => "warning",
            Kind::Note => "note",
        }
    }

    /// The letter of its codes: RCE, RCW, RCI.
    #[cfg(test)]
    fn letter(self) -> char {
        match self {
            Kind::Error => 'E',
            Kind::Warning => 'W',
            Kind::Note => 'I',
        }
    }
}

/// A code: what it is, its messages as written here (with their parameters), and its explanation.
#[derive(Debug)]
pub struct Code {
    pub id: &'static str,
    pub kind: Kind,
    pub title: &'static str,
    pub messages: &'static [&'static str],
    pub explanation: &'static str,
}

/// A message about a program: its code (none for the tool's own failures), the line, and the notes that go with
/// it (the opt-level 0 note under an error).
#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub code: Option<&'static Code>,
    pub message: String,
    pub notes: Vec<Diag>,
}

impl PartialEq for Code {
    fn eq(&self, other: &Code) -> bool {
        self.id == other.id
    }
}

impl Diag {
    fn new(code: &'static Code, message: String) -> Diag {
        Diag {
            code: Some(code),
            message,
            notes: vec![],
        }
    }

    /// A failure of the tool's own, not the program's: said as a plain `error:`, with no code.
    pub fn tool(message: impl Into<String>) -> Diag {
        Diag {
            code: None,
            message: message.into(),
            notes: vec![],
        }
    }

    pub fn kind(&self) -> Kind {
        self.code.map_or(Kind::Error, |c| c.kind)
    }

    /// Whether it is a message of this code.
    pub fn is(&self, code: &Code) -> bool {
        self.code == Some(code)
    }

    /// The message, with where it is from in front: `src/main.rs: ...`.
    pub fn at(mut self, place: &str) -> Diag {
        self.message = format!("{place}: {}", self.message);
        self
    }

    /// The message with more said after it: the function and the line it is about.
    pub fn and(mut self, more: &str) -> Diag {
        self.message.push_str(more);
        self
    }

    pub fn with_note(mut self, note: Diag) -> Diag {
        self.notes.push(note);
        self
    }

    /// Its code and its notes' codes.
    pub fn codes(&self) -> Vec<&'static Code> {
        self.code.into_iter().chain(self.notes.iter().flat_map(Diag::codes)).collect()
    }

    /// As `--message-format=json` says it: its kind in `reason`, as cargo's messages have, and its code.
    pub fn json(&self) -> Value {
        let mut v = json!({"reason": self.kind().word(), "message": self.message});
        if let Some(c) = self.code {
            v["code"] = c.id.into();
        }
        if !self.notes.is_empty() {
            v["children"] = self.notes.iter().map(Diag::json).collect::<Vec<_>>().into();
        }
        v
    }

    /// One read back from its JSON, as the driver passes it to `cargo cardiac`.
    pub fn from_json(v: &Value) -> Option<Diag> {
        let code = match v["code"].as_str() {
            Some(id) => Some(find(id)?),
            None => None,
        };
        let notes = match v["children"].as_array() {
            Some(n) => n.iter().map(Diag::from_json).collect::<Option<_>>()?,
            None => vec![],
        };
        Some(Diag {
            code,
            message: v["message"].as_str()?.to_string(),
            notes,
        })
    }
}

/// The lines for a person: `error[RCE0002]: ...`, and each note on a line of its own.
impl Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.code {
            Some(c) => write!(f, "{}[{}]: {}", c.kind.word(), c.id, self.message)?,
            None => write!(f, "error: {}", self.message)?,
        }
        for n in &self.notes {
            write!(f, "\n{n}")?;
        }
        Ok(())
    }
}

/// Where the codes are explained on the web: an index, and a page for each, PAGE/CODE/.
pub const PAGE: &str = "https://lab.strangequery.com/rust-cardiac/messages/";

/// rustc's last lines after its errors, which say where they are explained, for the codes of the errors and warnings
/// said (a note's never makes them): none when there are none.
pub fn more_information(said: &[&'static Code]) -> Option<String> {
    let mut codes: Vec<&Code> = said.iter().copied().filter(|c| c.kind != Kind::Note).collect();
    codes.sort_by_key(|c| c.id);
    codes.dedup();
    let first = codes.first()?;
    let word = first.kind.word();
    let (tool, id) = (crate::TOOL, first.id);
    let try_it = format!("try `cargo {tool} help {id}`, or see {PAGE}{id}/");
    if codes.len() == 1 {
        return Some(format!("For more information about this {word}, {try_it}"));
    }
    // an error's code sorts before a warning's, so the first is an error's when there are both
    let kinds = if codes.iter().all(|c| c.kind == first.kind) {
        format!("{word}s")
    } else {
        "errors and warnings".to_string()
    };
    let ids: Vec<&str> = codes.iter().map(|c| c.id).collect();
    let a = if first.kind == Kind::Error { "an" } else { "a" };
    Some(format!(
        "Some {kinds} have detailed explanations: {}.\nFor more information about {a} {word}, {try_it}",
        ids.join(", ")
    ))
}

/// The code of this name, in any case: `rce0002` is RCE0002.
pub fn find(id: &str) -> Option<&'static Code> {
    ALL.iter().copied().find(|c| c.id.eq_ignore_ascii_case(id))
}

/// Whether a word has a code's shape, in any case: RC, then E, W or I, then four digits. It may still be no code here.
pub fn looks_like_a_code(word: &str) -> bool {
    let w = word.as_bytes();
    w.len() == 7
        && w[..2].eq_ignore_ascii_case(b"RC")
        && matches!(w[2].to_ascii_uppercase(), b'E' | b'W' | b'I')
        && w[3..].iter().all(u8::is_ascii_digit)
}

/// The whole table as JSON, for the website: each code's kind, title, messages and explanation.
pub fn table_json() -> Value {
    let codes: Vec<Value> = ALL
        .iter()
        .map(|c| {
            json!({"code": c.id, "kind": c.kind.word(), "title": c.title, "messages": c.messages,
                "explanation": c.explanation})
        })
        .collect();
    json!({"tool": "cargo-cardiac", "version": env!("CARGO_PKG_VERSION"), "codes": codes})
}

/// The table: each entry its code, kind and title, its messages, and its explanation. Each code is a static of its
/// name, `ALL` lists them, and each message is a function of its name that makes the `Diag`.
macro_rules! codes {
    ($(
        $id:ident $kind:ident $title:literal {
            $( $f:ident ( $($p:ident : $t:ty),* ) $msg:literal ; )+
        }
        $explanation:literal
    )*) => {
        $(
            pub static $id: Code = Code {
                id: stringify!($id),
                kind: Kind::$kind,
                title: $title,
                messages: &[$($msg),+],
                explanation: $explanation,
            };
            $(
                pub fn $f($($p: $t),*) -> Diag {
                    Diag::new(&$id, format!($msg))
                }
            )+
        )*
        /// Every code, in the order of the table.
        pub static ALL: &[&Code] = &[$(&$id),*];
    };
}

codes! {
    RCE0001 Error "An internal error" {
        internal(what: impl Display) "internal: {what}";
    }
    r#"The compiler met something it should have handled and didn't: this is a bug in cargo-cardiac, not in your program.

The message says what the compiler was doing. Your program may be fine Rust that rustc accepts; the compiler took a wrong turn on the way to CARDIAC.

Please report it in [the issues](https://github.com/StrangeQuery/rust-cardiac/issues), with the smallest program that shows it and the command you ran. Until it is fixed, writing the same thing another way (a loop instead of an iterator, a local instead of a field) often gets past it.
"#

    RCE0002 Error "Rust the compiler doesn't model" {
        unsupported_statement(s: impl Display) "unsupported statement: {s}";
        unsupported_terminator(t: impl Display) "unsupported terminator: {t}";
        unsupported_call(callee: impl Display) "unsupported call to {callee}";
    }
    r#"A line of the MIR that rustc wrote for your program is one cargo-cardiac can't compile yet.

cargo-cardiac compiles rustc's MIR, the program after rustc has checked it and simplified it. It knows the statements and terminators that programs for CARDIAC need: arithmetic on cards, arrays, structs, enums, calls. The message quotes the MIR line it stopped at, which names what it is in Rust's terms (a call to `core::...` is a method of the standard library).

What to write instead depends on the line. Often it is a library method that hasn't been written for CARDIAC: write the loop or the test it stands for. The list of what compiles is in the repository's `docs/RUST.md`.

```rust
// in place of a library method the compiler doesn't have, the arithmetic it does
let n = v * v * v;
```
"#

    RCE0003 Error "A place CARDIAC can't address" {
        unsupported_place(p: impl Display) "unsupported place {p}";
        indexing_non_array(p: impl Display) "indexing a non-array {p}";
        outside_memory(p: impl Display) "unsupported place {p}: a pointer into memory this backend doesn't have (a static's is, when its first value is a literal of numbers or bools)";
    }
    r#"A place your program reads or writes (a local, a field, an element, what a pointer points to) is one the compiler can't give a cell.

Every value on CARDIAC lives in cells the compiler lays out before the program runs: a local is a cell, an array a row of cells, a struct a cell for each field. A pointer is the address of one of those cells. A place that is none of these, such as a pointer into memory the program didn't lay out, or an element of something that isn't an array, can't be reached.

Keep data in locals, arrays, structs and statics whose values are numbers or bools, and pass references to them.

```rust
// compiles: an array, passed by reference
fn total(t: &[i16; 4]) -> i16 {
    t[0] + t[1] + t[2] + t[3]
}
```
"#

    RCE0004 Error "A type no cell holds" {
        unsupported_type(ty: impl Display, local: impl Display) "unsupported local type {ty} for {local} (this backend has u8, i16, usize and bool)";
        field_type(f: impl Display, k: impl Display, s: impl Display, t: impl Display) "{f}: field {k} of {s} is {t}: fields must be scalars, structs or arrays of scalars";
        field_never(p: impl Display) "{p}: a field of a type with no values";
        field_unit(p: impl Display) "{p}: a field of type ()";
        enum_field_type(e: impl Display, k: impl Display, variant: impl Display, t: impl Display) "enum {e}: field {k} of {variant} is {t}: the fields of an enum's variants must be scalars (i16, u8, usize, bool), enums, structs or arrays of scalars for now";
    }
    r#"A value of this type can't be kept in CARDIAC's cells.

A cell holds a sign and three digits, -999 to 999. The types that fit are `i16`, `u8`, `usize` and `bool` (and `i8`), one cell each, and what is built of them: arrays, structs, tuples and enums. Floating point, the wide integers (`i32` and up, when a value of one is kept), and anything on the heap have no cell.

Use the card's own types. An `i16` holds every value a card can.

```rust
// refused: a float
let half: f32 = 0.5;
// compiles: the value in hundredths, on a card
let half: i16 = 50;
```
"#

    RCE0005 Error "A cast CARDIAC can't make" {
        unsupported_cast(src: impl Display) "unsupported cast: {src}";
    }
    r#"A cast (`as`) between types the compiler doesn't convert between.

Casts between the card's types compile: `u8 as i16`, `i16 as usize`, `bool as i16` and the like, with Rust's meaning (a `u8` wraps). A cast to or from a type no cell holds, or a pointer cast, doesn't.

Cast between `i16`, `u8`, `usize` and `bool`, or copy the value to a local first.

```rust
let b: u8 = t[i];
let v = b as i16;
```
"#

    RCE0006 Error "A constant of a type no cell holds" {
        unsupported_const(name: impl Display, ty: impl Display) "unsupported const {name}: {ty}";
    }
    r#"A `const` whose type has no cell: a float, or an integer type wider than a card.

A named `const` becomes the number it is, wherever it is used. A constant of `f32`, `f64`, or a wide integer the compiler can't narrow to a card, has no number on a card to become.

Give the constant a card's type, and its value in units a card can hold.

```rust
// refused
const SCALE: f64 = 2.5;
// compiles: tenths
const SCALE: i16 = 25;
```
"#

    RCE0007 Error "A function or constant the compiler doesn't have" {
        unknown_function(callee: impl Display) "call to unknown function {callee}";
        unknown_const(name: impl Display) "unknown const {name}";
    }
    r#"A call to a function, or a use of a constant, whose body the compiler was never given.

The compiler compiles the functions of your program and of the crates it uses. A function of the standard library that rustc calls rather than inlining (`core::...`) has no body here unless cargo-cardiac writes it itself, as it does for many of them. The same goes for a constant it can't find.

Write what the function does in your own code. A library method rustc inlines at opt-level 2 and calls at 1 may compile at 2.

```rust
// in place of a library method the compiler doesn't have: the loop it stands for
let mut s = 0;
while (s + 1) * (s + 1) <= a {
    s += 1;
}
```
"#

    RCE0008 Error "An operator on a type it isn't written for" {
        operator_type(op: impl Display, src: impl Display) "{op} is supported on i16, i8, u8 and usize, not here: {src}";
        method_type(name: impl Display, src: impl Display) "{name} is supported on integers: {src}";
        shift_type(name: impl Display, src: impl Display) "{name} by a variable is supported on i16 and u8, and >> on i8 and usize, not here: {src}";
        not_type(src: impl Display) "! is supported on bool, i16 and u8, not here: {src}";
    }
    r#"An operator used on a type the compiler has no code for it on.

Each operator is written for the types a card holds, and some for fewer: `!` on `bool`, `i16` and `u8`; `<<` by a variable on `i16` and `u8`; `>>` by a variable on `i16`, `i8`, `u8` and `usize`. On a type outside those (a `u16` that is a real value, an `i8`'s `<<`), the compiler stops rather than guess.

Do the operation on an `i16` or a `u8`, casting to it first.

```rust
let x: i8 = -3;
// refused: an i8's << by a variable
// let y = x << n;
let y = (x as i16) << n;
```
"#

    RCE0009 Error "A constant past what a card holds" {
        constant_past_card(v: impl Display, lo: impl Display, hi: impl Display) "constant out of range: {v} is past what a card holds ({lo} to {hi}): a literal that size, or arithmetic rustc worked out to it at compile time, would overflow a card";
        limit_past_card(name: impl Display, v: impl Display) "constant out of range: i16::{name} ({v}) is past what a card holds (-999 to 999): no i16 on a card reaches it, so code that uses it as a value (a saturating method, say) can't be compiled";
        cast_past_card(v: impl Display, to: impl Display, src: impl Display) "constant out of range: {v} as {to} is past what a card holds, in {src}";
        enum_past_card(e: impl Display, variant: impl Display, v: impl Display) "enum {e}: {variant} is {v}, past what a card holds (-999 to 999)";
        static_past_card(name: impl Display, v: impl Display) "static {name}: {v} is past what a card holds (-999 to 999)";
        dat_past_card(x: impl Display) "constant out of range: DAT {x} is past what a card holds";
    }
    r#"A number in the program that no card can hold: past -999 to 999.

A cell holds a sign and three digits. A literal like `1010`, an enum variant numbered past 999, a `static` of one, or arithmetic rustc works out at compile time to such a number (`101 * 10`, even in a branch no run takes) has nowhere to go. It is Rust's own rule for a value that can't fit, for a card instead of an `i16`: rustc refuses `30000 * 2` in an `i16` and `i16::MAX + 1` in a `const`.

`i16::MAX` (32767) and `i16::MIN` are such numbers too. Methods that use them as values, such as `saturating_add`, which stops at 32767, can't be compiled.

Keep constants on a card. Compute what may pass one at run time, where the overflow check sees it.

```rust
// refused: 1200 is past a card
const LIMIT: i16 = 1200;
// compiles
const LIMIT: i16 = 999;
```
"#

    RCE0010 Error "Arithmetic in i32 that can pass a card" {
        widened(method: impl Display, f: impl Display) "{method}, inlined in {f}, works in i32 so that it can't overflow, and on CARDIAC an i32 is a card (-999 to 999), past which it can: it is not supported";
    }
    r#"A library method that does its arithmetic in a wider type, so that it can't overflow, and on CARDIAC can.

Some methods of `i16`, such as `midpoint`, work in `i32`: `(a as i32 + b as i32) / 2` never overflows. On CARDIAC every integer is a card, `i32` too, so the sum in the middle can pass 999 where Rust's can't, and the answer would be wrong.

Write the arithmetic so that it stays on a card.

```rust
// refused
let m = a.midpoint(b);
// compiles, where a + b stays on a card
let m = (a + b) / 2;
```
"#

    RCE0011 Error "A u8 product with its overflow flag" {
        overflowing_mul_u8() "a u8 product with its overflow flag (`overflowing_mul`, `checked_mul`) is not supported: past a card it can't be wrapped";
    }
    r#"`overflowing_mul` or `checked_mul` on a `u8`, whose product can pass what a card holds.

A `u8` product can be as big as 255 × 255. The method needs the whole product to say whether it overflowed and to wrap it, and on CARDIAC a product past 999 can't be held to be wrapped.

Multiply in `i16` with values that keep the product on a card, and test it yourself.

```rust
let p = a as i16 * b as i16; // a and b below 32: on a card
let fits = p <= 255;
```
"#

    RCE0012 Error "A wrapping product past the accumulator" {
        wrapping_mul_wide(src: impl Display) "a wrapping * of two values that can pass what the accumulator holds is not supported: {src}";
    }
    r#"A wrapping multiplication (`wrapping_mul`, or `*` without overflow checks) of two `u8`s or `i8`s whose product could pass what CARDIAC's accumulator holds. The compiler no longer says this: such a multiplication compiles.

CARDIAC has no multiply: a product is a loop of adds, and where it passes a card the machine's own rule says what is left, as for a sum. A cell keeps the last three digits and the sign (999 + 1 stores 000), and the `u8` or `i8` then wraps those as Rust wraps any value. A product from -999 to 999 wraps just as Rust's does; past that, the value is CARDIAC's, not Rust's.

With overflow checks on, a product past the type's range panics, as Rust's does.

```rust
let y = a.wrapping_mul(b); // 200 × 200 = 40000 is stored as 000: 0, where Rust has 64
```
"#

    RCE0013 Error "Recursion" {
        recursion(path: impl Display) "recursion ({path}) needs a stack, which only a function calling itself gets here";
    }
    r#"Recursion the compiler can't give a stack: through more than one function, or of a function whose locals it can't push.

Each function on CARDIAC has cells of its own for its locals, laid out once: there is no stack. A second call of a function before the first returns would write over the first's cells.

A function that calls itself as the last thing it does compiles: the call becomes a loop. So do functions that call one another as the last thing each does (`is_even` and `is_odd`): they become one function. A function that calls itself anywhere else gets a stack of its own, in the memory the program leaves over: the locals it still reads after the call are pushed before it and popped after, and a call past the stack's end panics with `stack overflow`. That takes about 30 cells, and only where its locals and its result are numbers or bools with no reference among them. Recursion through two or more functions, other than at their ends, and recursion of a function that returns a struct or holds a reference, are refused: write them as a loop.

```rust
// compiles, with a stack (deep inputs panic: stack overflow)
fn sum(n: i16) -> i16 {
    if n == 0 { 0 } else { n + sum(n - 1) }
}
// refused: f calls g, and g calls f, other than at the end
fn f(n: i16) -> i16 { if n == 0 { 0 } else { 1 + g(n - 1) } }
fn g(n: i16) -> i16 { f(n) * 2 }
// compiles, and needs no stack at all
fn sum_loop(mut n: i16) -> i16 {
    let mut s = 0;
    while n > 0 {
        s += n;
        n -= 1;
    }
    s
}
```
"#

    RCE0014 Error "A generic function from another crate" {
        generic_in_crate(within: impl Display, callee: impl Display, args: impl Display) "{within}call to {callee}{args}, a generic function in another crate: make it non-generic, or #[inline] so rustc inlines it";
    }
    r#"A call to a generic function of another crate, which rustc has no MIR of for these types.

rustc writes a crate's MIR once, in its generic form. The compiler needs each function for the types it is called with, and for a function of another crate it only has the generic one.

Make the function non-generic, or mark it `#[inline]`, so rustc copies it into your program for your types.

```rust
// in the other crate
#[inline]
pub fn larger<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```
"#

    RCE0015 Error "A call that could be one of several functions" {
        ambiguous_call(within: impl Display, callee: impl Display, which: impl Display) "{within}call to {callee} could be any of {which}";
    }
    r#"A call names a function that more than one of the crates the program uses has.

The compiler links your program and its crates into one, finding each function by its path. A path that two crates both have (each a function of the same name at its root) can't be told apart.

Rename one of the functions, or call it by its whole path.

```rust
let v = kit::scale(x);
```
"#

    RCE0016 Error "A constant defined in two crates" {
        const_in_two_crates(name: impl Display, one: impl Display, other: impl Display) "const {name} is defined in both {one} and {other}: rename one";
    }
    r#"Two crates the program uses each define a `const` of the same name.

The program's constants and its crates' are linked by name, so two of one name would be one constant with two values.

Rename one of them.

```rust
pub const KIT_LIMIT: i16 = 500;
```
"#

    RCE0017 Error "An enum with data used in a way not supported yet" {
        enum_constant(f: impl Display, line: impl Display) "{f}: {line}: a constant of an enum with data is not supported yet";
        enum_use(f: impl Display, line: impl Display) "{f}: {line}: an enum with data used this way is not supported yet";
        enum_returned(f: impl Display, line: impl Display) "{f}: {line}: an enum with data returned into this place is not supported yet";
        enum_other_variant(f: impl Display, line: impl Display) "{f}: {line}: a variant of another enum";
        enum_projection(p: impl Display) "{p}: a projection of an enum with data that isn't a variant's field";
        enum_no_variant(p: impl Display) "{p}: no such variant or field";
    }
    r#"An enum with data (`Option<i16>`, `Result`, an enum of your own with fields) used in a way the compiler doesn't lay out yet.

An enum with data is a tag cell, the variant's number, and cells for its fields, which the variants share. Building one, matching on it, passing and returning it compile. Some uses don't yet: an enum constant rustc writes as raw bytes, an enum with data copied into some places, nested in ways the layout doesn't follow.

Build the value where it is used, and take it apart with `match` or `if let`. A struct of a tag and the fields does the same job.

```rust
let r = if ok { Some(v) } else { None };
match r {
    Some(v) => out(v),
    None => out(-1),
}
```
"#

    RCE0018 Error "A struct used in a way not supported yet" {
        struct_behind_ref(f: impl Display, l: impl Display, ty: impl Display) "{f}: {l}: {ty}: a struct behind a reference is not supported yet";
        struct_array_with_arrays(f: impl Display, l: impl Display, ty: impl Display) "{f}: {l}: {ty}: an array of structs with arrays in them is not supported yet";
        struct_array_param(f: impl Display) "{f}: an array of structs as a parameter is not supported yet";
        struct_array_use(f: impl Display, s: impl Display) "{f}: {s}: an array of structs used this way is not supported yet";
        struct_whole_use(f: impl Display, s: impl Display) "{f}: {s}: a whole struct used this way is not supported yet";
        struct_element_assignment(f: impl Display, line: impl Display) "{f}: {line}: unsupported element assignment";
        struct_copy(f: impl Display, line: impl Display) "{f}: {line}: unsupported struct copy";
        struct_result_elsewhere(f: impl Display, line: impl Display) "{f}: {line}: a struct result from a function that is not compiled here";
        struct_field_past_end(f: impl Display, k: impl Display, s: impl Display) "{f}: field {k} of {s} is past its last field";
    }
    r#"A struct used in a way the compiler doesn't take apart yet.

A struct on CARDIAC is a cell for each field, and an array of structs an array for each field. The compiler follows each use of a struct to those cells: a field read or written, the whole struct passed, returned or copied. Some uses it can't follow yet: a struct with an array in it behind a reference or in an array, an array of structs passed as a parameter, a whole struct read in some places.

Pass the fields you need, pass the struct by value, or keep parallel arrays, one for each field.

```rust
struct Point { x: i16, y: i16 }
// parallel arrays in place of [Point; 8]
let mut xs = [0i16; 8];
let mut ys = [0i16; 8];
```
"#

    RCE0019 Error "A slice whose length isn't known" {
        slice_result_length(f: impl Display) "{f}: it returns a slice whose length isn't known here";
        slice_argument_length(f: impl Display, callee: impl Display) "{f}: a call of {callee}: the length of the slice it is passed isn't known here";
    }
    r#"A slice whose length the compiler can't follow to where it is used.

A slice is an address and a length. The compiler keeps the length in a cell beside the address, and follows it from where the slice is made (an array, a range of one) through the calls it is passed to and returned from. Where it loses the length, it can't check an index or end a loop.

Make the slice from an array where it is used, or pass the array by reference, `&[i16; N]`.

```rust
fn total(t: &[i16; 8]) -> i16 {
    let mut s = 0;
    for i in 0..8 {
        s += t[i];
    }
    s
}
```
"#

    RCE0020 Error "An array passed by value that isn't a local" {
        array_by_value(callee: impl Display) "call to {callee}: an array passed by value must be a local array; pass it by reference, `&[T; N]`";
    }
    r#"An array passed by value that isn't a local array of the caller's.

An array passed by value is copied, element by element, from the caller's array into the function's. The compiler copies from a local array; an array that is a constant, a field, or the result of an expression has to be a local first.

Pass it by reference, which copies nothing, or put it in a local first.

```rust
fn total(t: &[i16; 4]) -> i16 { t[0] + t[1] + t[2] + t[3] }
let s = total(&[1, 2, 3, 4]);
```
"#

    RCE0021 Error "`skip_cards()` given the wrong arguments" {
        skip_cards_arguments() "skip_cards() takes the number of cards: skip_cards(n)";
        skip_cards_range(n: impl Display) "skip_cards({n}): n is 0 to 999";
    }
    r#"`skip_cards(n)` called with something other than a number of cards from 0 to 999.

`skip_cards(n)` reads n cards and drops them, as `for _ in 0..n { inp(); }` does, with a jump into a row of `INP` instructions. The row covers 0 to 999 cards.

Give it one number, 0 to 999.

```rust
let n = inp();
skip_cards(n);
```
"#

    RCE0022 Error "`add_times()` given the wrong arguments" {
        add_times_arguments() "add_times() takes the value and how many times: add_times(c, n)";
        add_times_range(n: impl Display) "add_times(c, {n}): n is 0 to 99";
    }
    r#"`add_times(c, n)` called with something other than a value and a count from 0 to 99.

`add_times(c, n)` is c added up n times, `n * c`, with a jump into a row of `ADD` instructions, once for the tens of n and once for the units. n is 0 to 99, and the product must fit on a card.

```rust
let total = add_times(price, count); // count 0 to 99
```
"#

    RCE0023 Error "A resource name that isn't a literal name" {
        resource_not_literal() "load_resource() takes the resource's name as a string literal: load_resource(\"key\")";
        resource_bad_name(name: impl Display) "resource {name}: a resource's name is letters, digits and _, the name of a file in resources/";
    }
    r#"`load_resource` given a name that isn't a string literal, or a literal that isn't a plain file name.

A resource is the file `resources/NAME` beside the program, which the loader puts in memory with the program. The compiler names the cells in the deck by the resource's name, so the name must be known when it compiles: a string literal of letters, digits and `_`.

```rust
let table: [i16; 10] = load_resource("squares");
```
"#

    RCE0024 Error "A resource loaded twice" {
        resource_twice(name: impl Display) "resource {name} is loaded twice: load it once and pass the array on";
    }
    r#"The same resource loaded by two calls of `load_resource`.

The loader fills a resource's cells once, when the deck is read. A second call would be a second array with the same cells.

Load it once, in `main`, and pass the array (by reference) to whatever needs it.

```rust
let t: [i16; 10] = load_resource("squares");
show(&t);
sum(&t);
```
"#

    RCE0025 Error "A resource loaded outside main, or in a loop" {
        resource_outside_main(name: impl Display) "load_resource(\"{name}\") is only allowed in main, where it runs once: load it there and pass the array on";
        resource_in_loop(name: impl Display) "load_resource(\"{name}\") is in a loop: the loader fills it once, so load it before the loop";
    }
    r#"`load_resource` called where it can run more than once: in a function other than `main`, or in a loop.

A resource is filled once, by the loader, before the program starts. Natively, `load_resource` reads the file each time it is called; on CARDIAC there is no second reading. So the call has to be where it runs once: in `main`, before any loop.

```rust
fn main() {
    let t: [i16; 10] = load_resource("squares");
    loop {
        let i = inp() as usize;
        out(t[i]);
    }
}
```
"#

    RCE0026 Error "A resource not bound to an array" {
        resource_unbound(name: impl Display) "load_resource(\"{name}\") must be bound with let: `let t: [i16; N] = load_resource(\"{name}\");`";
        resource_not_array(name: impl Display) "load_resource(\"{name}\") must be bound to an array with let: `let t: [i16; N] = load_resource(\"{name}\");`";
    }
    r#"The result of `load_resource` not bound with `let` to an array.

The resource's cells are an array of the program's, which the loader fills. The array's length, which says how many cells, comes from the type of the `let` it is bound to.

```rust
let t: [i16; 10] = load_resource("squares");
```
"#

    RCE0027 Error "`overlay()` outside main" {
        overlay_outside_main() "overlay() is only allowed in main";
    }
    r#"`overlay()` called in a function other than `main`.

A program too big for memory comes in phases, each read in over the one before; `overlay()` is where one phase ends and the next is loaded. A phase is a stretch of `main`: the functions it calls are in it, and can't end it.

Call `overlay()` in `main`, between the parts of the program.

```rust
fn main() {
    read_table();
    overlay();
    print_report();
}
```
"#

    RCE0028 Error "More phases than CARDIAC can load" {
        too_many_phases() "more phases than the bug can name (tapes 1 to 98)";
    }
    r#"A program with more phases than the loader can number.

Each phase is a deck of its own, and the instruction that ends a phase names the next one by a number on a card, 1 to 98.

Use fewer `overlay()` calls: each phase can be as big as memory.
"#

    RCE0029 Error "`overlay()` with auto-overlay on" {
        overlay_with_auto_overlay() "overlay() in main with auto-overlay on: the compiler places the phases itself, so take the overlay() out or turn auto-overlay off";
    }
    r#"`overlay()` in a program that has auto-overlay on.

With auto-overlay on (`auto-overlay = true` under `[package.metadata.cardiac]`, or `--auto-overlay` for a single file), the compiler cuts `main` into phases itself, where it fits best. An `overlay()` of the program's own would be a second say in the same thing.

Take the `overlay()` calls out, or turn auto-overlay off and place them by hand.
"#

    RCE0030 Error "`overlay()` in a loop" {
        overlay_in_loop(phase: impl Display) "phase {phase} ends at an overlay() that an earlier phase ended at: overlay() in a loop";
    }
    r#"An `overlay()` that the program can reach more than once.

Each `overlay()` ends one phase and loads the next, which is read in over the code before it. Reached a second time, it would load the same phase again, with the code that came after it gone.

Put `overlay()` where `main` runs once, outside any loop.

```rust
fn main() {
    let n = inp();
    overlay();
    for i in 0..n {
        out(i);
    }
}
```
"#

    RCE0031 Error "A phase that reaches into another" {
        phase_two_overlays(phase: impl Display) "phase {phase} ends at more than one overlay()";
        phase_jumps(phase: impl Display, to: impl Display) "phase {phase} jumps to {to}, in another phase";
    }
    r#"A phase whose code goes on into another phase other than through its `overlay()`.

Only one phase's code is in memory at a time. A phase has to end at one `overlay()`, and nothing in it may jump to code of another phase, which isn't there. An `overlay()` in one arm of an `if` and another in the other arm, or a loop that crosses an `overlay()`, breaks that.

Place each `overlay()` where every path through `main` passes it once.

```rust
fn main() {
    let a = inp();
    let b = if a < 0 { -a } else { a };
    overlay(); // after the if, not in its arms
    out(b);
}
```
"#

    RCE0032 Error "Too big for CARDIAC's memory" {
        too_big(n: impl Display) "{n} cells, more than CARDIAC's memory holds (96, cells 03 to 98)";
        runs_past_98() "program runs past cell 98";
    }
    r#"The program, or one of its phases, needs more cells than CARDIAC has.

CARDIAC has 100 cells. Cells 00 to 02 and 99 belong to the machine and the loader, which leaves 96 for the program's code and data.

Make it smaller, or split it into phases: `overlay()` in `main` where it can be cut, or auto-overlay on to have the compiler cut it. Things that cost many cells are listed in `docs/RUST.md` (multiplying or dividing two variables, the bit routines, overflow checks); `cargo cardiac build -v` says where the cells go.

```toml
[package.metadata.cardiac]
auto-overlay = true
```
"#

    RCE0033 Error "No `fn main`" {
        no_main() "no fn main in MIR";
    }
    r#"The program has no `fn main`.

A CARDIAC program starts at `main`, as a Rust program does. A program built as a single file needs one too.

```rust
fn main() {
    out(inp());
}
```
"#

    RCE0034 Error "`println!` and `print!`" {
        println(why: impl Display) "println! and print! aren't supported on CARDIAC, which prints only numbers: use out() ({why})";
    }
    r#"`println!` or `print!` in a program for CARDIAC.

CARDIAC's output is a teletype that prints numbers, one a line. There is no text to print, and the formatting machinery behind `println!` is far bigger than memory.

Output numbers with `out()`.

```rust
// refused
println!("{}", total);
// compiles
out(total);
```
"#

    RCE0035 Error "`format!` and `{}` formatting" {
        format(why: impl Display) "format! and {{}} formatting aren't supported on CARDIAC, which has no text: print numbers with out() ({why})";
    }
    r#"`format!`, `write!` or another use of Rust's `{}` formatting.

Formatting makes text, and CARDIAC has none: a cell holds a number, and the teletype prints numbers.

Output the numbers themselves with `out()`.

```rust
out(hours);
out(minutes);
```
"#

    RCE0036 Error "`String`" {
        string(why: impl Display) "String isn't supported on CARDIAC, which has no text and no heap ({why})";
    }
    r#"A `String` in a program for CARDIAC.

A `String` lives on the heap, and CARDIAC has no heap (and no text). A string literal's bytes (`b"..."`, or a `&str` literal's `as_bytes()`) compile, as an array of numbers.

Use numbers, or an array of them.

```rust
let word = b"HI";
out(word[0] as i16);
```
"#

    RCE0037 Error "`Vec`" {
        vec(why: impl Display) "Vec isn't supported on CARDIAC, which has no heap: use a fixed-size array ({why})";
    }
    r#"A `Vec` in a program for CARDIAC.

A `Vec` grows on the heap. CARDIAC has 96 cells and no heap: every array's size is known before the program runs.

Use a fixed-size array, and a count of how much of it is used.

```rust
let mut t = [0i16; 20];
let mut n = 0;
loop {
    let v = inp();
    if v < 0 { break; }
    t[n] = v;
    n += 1;
}
```
"#

    RCE0038 Error "An iterator the compiler doesn't have" {
        iterator(why: impl Display) "iterators other than ranges aren't supported: index the array in a `for i in 0..N` loop ({why})";
    }
    r#"An iterator, or an iterator adapter, the compiler doesn't turn into a loop.

The compiler writes many of the iterators rustc leaves as calls as the loops they stand for: ranges, `for x in array`, `.iter()` and some of its methods. Others, and chains of adapters, it doesn't have yet.

Index the array in a `for` loop over a range.

```rust
// refused
let total: i16 = t.iter().map(|v| v * 2).sum();
// compiles
let mut total = 0;
for i in 0..t.len() {
    total += t[i] * 2;
}
```
"#

    RCE0039 Error "A trait object used this way" {
        trait_object(why: impl Display) "this use of a trait object isn't supported: a `&dyn Trait` works where it is made of a reference to one of the program's structs, and a method called through it is one each struct's impl defines, and a `&dyn Fn(..)` where it is made of a closure that captures ({why})";
    }
    r#"A trait object (`&dyn Trait`, `Box<dyn Trait>`) used in a way the compiler can't follow.

A `&dyn Trait` compiles as an enum of the program's types that are made into one, and a method call through it as a `match` on which type it is. That needs every such object to be a reference to one of the program's own structs, and each method one their impls define. A `&dyn Fn(..)` is the same with closures: each one made into it must capture something (a closure that captures nothing can be a function pointer, `fn(i16) -> i16`). A `Box<dyn Trait>`, a `&mut dyn FnMut(..)`, or a trait object of a library type, isn't.

Use a `&dyn Trait` of your own structs, or an enum and a `match`.

```rust
enum Shape { Square(i16), Line(i16) }
fn area(s: &Shape) -> i16 {
    match s {
        Shape::Square(a) => a * a,
        Shape::Line(_) => 0,
    }
}
```
"#

    RCE0040 Error "A slice the compiler can't follow" {
        slice(why: impl Display) "slices aren't supported: pass an array by reference, `&[i16; N]` ({why})";
    }
    r#"A slice (`&[T]`) used in a way the compiler can't compile.

A slice is an address and a length. The compiler compiles many slices (of an array, passed to a function, its length and its elements), but not every use the standard library makes of one.

Pass the array by reference, `&[i16; N]`, whose length is part of its type.

```rust
fn total(t: &[i16; 8]) -> i16 {
    let mut s = 0;
    for i in 0..8 { s += t[i]; }
    s
}
```
"#

    RCE0041 Error "A single file that uses the `cardiac` crate" {
        single_file_cardiac(file: impl Display) "{file}: a single file can't use the cardiac crate: build it in a package, or define the built-ins it calls in the file itself";
    }
    r#"A program built as a single file (`cargo cardiac build FILE.rs`) that uses the `cardiac` crate.

A single file is compiled by rustc on its own, with no Cargo package to say where the `cardiac` crate is.

Make it a package with `cargo cardiac new`, which depends on `cardiac`, or define the built-ins the file calls in the file itself, as stubs the compiler recognises by name:

```rust
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
```
"#

    RCE0042 Error "A crate with no library" {
        no_library(package: impl Display, krate: impl Display) "{package}'s binaries can't be compiled for CARDIAC: it uses {krate}, a crate with no library (a proc-macro crate, say), which can't be compiled for CARDIAC";
    }
    r#"The package uses a crate that has no library, such as a proc-macro crate.

cargo-cardiac compiles the program and the libraries it uses for CARDIAC. A proc-macro crate runs inside rustc on your computer and has no library to compile.

Use crates that are plain libraries. A crate of macros made with `macro_rules!` is one.
"#

    RCW0001 Warning "An expensive operation" {
        expensive(ops: impl Display) "expensive operation, consider an alternative: {ops}";
    }
    r#"The program uses a bit operation, which on CARDIAC is a long routine or a loop over the bits.

A cell holds a decimal number, not bits. `&`, `|` and `^` between two values, `<<` and `>>` by a variable, and the bit methods (`count_ones`, `leading_zeros`, `trailing_zeros`, `rotate_left`, `rotate_right`) each walk a value's bits one at a time. The routines for `&`, `|` and `^` are 50 to 70 instructions each, put in once, and take about 200 steps a call; the bit methods are 20 to 40 cells where each is called. A program that uses one has far less of its 96 cells left. The program is right; it is just big and slow.

The warning comes once a program, naming each bit operation it uses. Multiplying and dividing two variables also call routines, and aren't warned about: they are what arithmetic costs on CARDIAC.

Decimal arithmetic is usually the alternative: `/` and `%` by a power of ten are shifts, `digit()` takes one digit, and `x & m` by a mask one below a power of two is a remainder, which needs no routine.

```rust
// flags packed in bits: a routine for &, and a shift by a variable
if flags & (1 << k) != 0 {
    out(k);
}
// flags packed in digits (k is 0, 1 or 2): one shift
if digit(flags, k) == 1 {
    out(k);
}
```
"#

    RCI0001 Note "Bools that would be smaller as `Flag`s" {
        flag_advice(which: impl Display, n: impl Display, saved: impl Display) "{which} tested {n} times; as cardiac::Flag (`false.into()`, `.get()`) each test is an instruction less: about {saved} cells or more";
    }
    r#"The program keeps `bool`s in cells and tests them often enough that `cardiac::Flag` would make it smaller.

CARDIAC has one test, TAC: jump if the accumulator is negative. A `bool` is 0 or 1, as Rust promises, so testing one kept in a cell is three instructions: load it, take away 1, TAC. A `cardiac::Flag` holds -1 for false and 0 for true, so testing it is two. That is a cell and a step less a test, for one cell holding -1, which many programs have already.

A condition tested where it is worked out (`if a < b`) is never kept in a cell, and needs no `Flag`.

```rust
use cardiac::*;

let mut seen_zero: Flag = false.into();
loop {
    let v = inp();
    if v == 0 {
        seen_zero = true.into();
    }
    if seen_zero.get() {
        out(v);
    }
}
```
"#

    RCI0002 Note "Auto-overlay split `main` into phases" {
        split_into_phases(n: impl Display) "main doesn't fit in memory in one piece, so auto-overlay split it into {n} phases";
    }
    r#"With auto-overlay on, the compiler cut `main` into phases, because it doesn't fit in memory in one piece.

Each phase is a deck of its own (NAME.cqb, NAME.2.cqb, ...), loaded over the code of the one before; the data stays. The compiler tries each place where `main` runs once, and keeps the split that leaves the smallest program with every phase in memory.

Nothing needs changing. `cargo cardiac run` runs the phases in turn.
"#

    RCI0003 Note "Opt-level 0" {
        opt_level_zero() "this is opt-level 0, where rustc's MIR is unoptimized and cargo-cardiac's passes are off; most programs need opt-level 1 or more";
    }
    r#"The program was built at opt-level 0, where few programs compile for CARDIAC.

At opt-level 0 rustc writes its MIR unoptimized (an iterator's `next` is a call, every small helper is a call), and cargo-cardiac's own passes are off. Few programs come out small enough for 96 cells, and some constructs only compile once rustc has inlined them.

Set the opt-level to 1 or more: `opt-level` in the Cargo profile, or `-C opt-level=2` for a single file. Building a package, `cargo cardiac` sets it for the dev profile when Cargo.toml doesn't.

```toml
[profile.dev]
opt-level = 2
```
"#

    RCI0004 Note "Dev profile settings chosen for CARDIAC" {
        dev_defaults(settings: impl Display) "building with {settings} in the dev profile, which [profile.dev] doesn't set";
    }
    r#"`cargo cardiac` built the dev profile with settings of its own, because the package's Cargo.toml doesn't set them.

Cargo's dev profile doesn't suit CARDIAC as it is. CARDIAC needs rustc's optimized MIR (opt-level 1 or more), the CARDIAC driver can't build incrementally, and debug info changes the listing. The checks rarely fit in 96 cells, so overflow checks and debug assertions are off: an overflow that `cargo run` panics on goes unchecked on CARDIAC.

Set any of them in `[profile.dev]` to choose your own, and the note goes.

```toml
[profile.dev]
opt-level = 2
incremental = false
debug = false
overflow-checks = false
debug-assertions = false
```
"#

    RCI0005 Note "A resource with no file yet" {
        resource_empty(deck: impl Display, name: impl Display, file: impl Display) "{deck}: resource {name} has no file {file}; its slot in the deck is empty until it is bound";
    }
    r#"The program loads a resource whose file isn't there yet, so its slot in the deck is empty.

`load_resource("NAME")` is filled from the file `resources/NAME`. When the file is there at build time, its numbers go in the deck. When it isn't, the deck has an empty slot, which `cargo cardiac run` fills from `resources/` beside the deck when it runs it.

Put the file in `resources/` to have it built into the deck: numbers, written as the input is.
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_is_numbered_from_one_with_no_gaps() {
        for kind in [Kind::Error, Kind::Warning, Kind::Note] {
            let ids: Vec<&str> = ALL.iter().filter(|c| c.kind == kind).map(|c| c.id).collect();
            for (k, id) in ids.iter().enumerate() {
                assert_eq!(*id, format!("RC{}{:04}", kind.letter(), k + 1), "{id}");
            }
        }
    }

    #[test]
    fn every_code_has_a_title_a_message_and_an_explanation() {
        for c in ALL {
            assert!(!c.title.is_empty() && !c.messages.is_empty(), "{}", c.id);
            let first = c.explanation.lines().next().unwrap_or("");
            assert!(first.ends_with('.') && !first.starts_with('#'), "{}: {first}", c.id);
            assert_eq!(c.explanation.matches("```").count() % 2, 0, "{}", c.id);
            for m in c.messages {
                assert!(!m.contains('\n') && !m.ends_with('.'), "{}: {m}", c.id);
            }
        }
    }

    #[test]
    fn a_message_says_its_code_and_reads_back_from_json() {
        let e = unsupported_statement("_1 = foo")
            .at("src/main.rs")
            .with_note(opt_level_zero());
        assert_eq!(
            e.to_string().lines().next(),
            Some("error[RCE0002]: src/main.rs: unsupported statement: _1 = foo")
        );
        assert!(e.to_string().contains("\nnote[RCI0003]: this is opt-level 0"));
        assert_eq!(Diag::from_json(&e.json()), Some(e.clone()));
        assert_eq!(e.json()["code"], "RCE0002");
        assert_eq!(find("rce0002").map(|c| c.id), Some("RCE0002"));
        assert_eq!(Diag::tool("no driver").to_string(), "error: no driver");
        assert_eq!(e.codes().iter().map(|c| c.id).collect::<Vec<_>>(), ["RCE0002", "RCI0003"]);
    }

    #[test]
    fn the_last_lines_say_where_the_codes_are_explained() {
        assert_eq!(more_information(&[&RCI0003]), None);
        assert_eq!(
            more_information(&[&RCE0013, &RCI0003]).as_deref(),
            Some(
                "For more information about this error, try `cargo cardiac help RCE0013`, or see \
                 https://lab.strangequery.com/rust-cardiac/messages/RCE0013/"
            )
        );
        assert_eq!(
            more_information(&[&RCW0001, &RCE0013, &RCE0002, &RCE0013]).as_deref(),
            Some(
                "Some errors and warnings have detailed explanations: RCE0002, RCE0013, RCW0001.\n\
                 For more information about an error, try `cargo cardiac help RCE0002`, or see \
                 https://lab.strangequery.com/rust-cardiac/messages/RCE0002/"
            )
        );
        assert!(more_information(&[&RCW0001]).unwrap().starts_with("For more information about this warning"));
        assert!(looks_like_a_code("rcw0001") && looks_like_a_code("RCE9999"));
        assert!(!looks_like_a_code("RCX0001") && !looks_like_a_code("RCE001") && !looks_like_a_code("build"));
    }
}
