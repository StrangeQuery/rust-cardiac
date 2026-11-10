//! Generated programs (stage 2): small random Rust programs in the subset the compiler supports, each
//! built in every setting and run on generated inputs as the programs of tests/fuzz/targets are, so that the
//! compiler's passes see code no one wrote by hand.
//!
//! A program is made from a seed and its number, the same on every machine: up to three inputs, each read with
//! inp() from a range of its own (some kept by an assume() the range keeps), a few helper functions (some
//! #[inline(never)], one through a `&mut i16`), constants and constant tables, and a main of a few statements:
//! locals and assignments, if/else, while loops with counters and for loops over ranges, early breaks, local
//! arrays written and read, out(). The expressions lean to what the passes rewrite: sums and differences in
//! chains, multiplication by constants, `/` and `%` by constants and by variables (which can be 0), comparisons
//! with 0, ±1 and the card's edges, if/else expressions with the same value on both sides, an expression used
//! again, tables indexed by computed values in and out of bounds. Each value's range is followed roughly as the
//! program is made, so that most values stay on a card, and most locals are printed, so few are dead.
//!
//! Some programs (`--enums PCT` of them, 30 by default; 0 makes each seed's programs as before) also have enums,
//! each a kit of types and helpers used by expressions of a few forms: a C-like enum, matched and cast; an
//! enum with data (a unit variant, one and two i16 fields, a u8 and a bool) built by one helper and matched by
//! another; an `Option<i16>` returned, matched, `if let`, `unwrap_or`, `is_some`, `map`; and a `Result<i16, i16>`
//! with `?` in an #[inline(never)] helper, matched, `unwrap_or`, `is_ok`, `ok()`.
//!
//! The generator also runs each program itself, on each input, as Rust would but with numbers of any size: what it
//! prints, how it ends, whether any value it kept passed a card on the way, and whether any value did. That is the
//! oracle for "past a card". Without overflow checks, a run in which no value kept (in a variable, an array, a
//! parameter or a result, a loop's bounds or an index, or output) passed one must print the same on CARDIAC,
//! whatever the passes folded away: a value past a card only inside an expression, back on one before anything
//! keeps it, is held whole in the accumulator, as in native Rust. With overflow checks, CARDIAC stops at any value
//! past a card, so a run must match only if no value passed one. Any other run is no finding. Its cards are
//! checked against Rust's on every run, and a disagreement is counted (the generator's mistake, never a finding).
//!
//! The programs are built a batch at a time, as the binaries of one package (target/fuzz/gen/pkg): one cargo
//! build a setting for the batch natively, and one `cargo cardiac build` for all of them, again for the rest when
//! one fails. A program rustc refuses is the generator's mistake, counted; one too big for CARDIAC's 96 cells is
//! counted and passed over, as is one refused for a constant past a card (an error, as Rust's overflow at compile
//! time is). Any other
//! refusal, a compiler that panics, overflows its stack or hangs, and a run that differs with no value past a card,
//! is a finding: the first two of each kind shrunk (the input, then the program a statement or an expression at a
//! time, a round of candidates built together in target/fuzz/shrink/pkg) and saved, the program as generated and
//! as shrunk and what was found, in target/fuzz/findings/.
use super::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// A value's possible range, as far as the generator follows it.
type Iv = (i64, i64);

const CARD: Iv = (-999, 999);

fn fits(iv: Iv) -> bool {
    iv.0 >= CARD.0 && iv.1 <= CARD.1
}

fn join(a: Iv, b: Iv) -> Iv {
    (a.0.min(b.0), a.1.max(b.1))
}

/// Kept within what i16 arithmetic on the generator's side can say: a range this wide says "anything".
fn cap(iv: Iv) -> Iv {
    (iv.0.clamp(-40000, 40000), iv.1.clamp(-40000, 40000))
}

fn mag(iv: Iv) -> i64 {
    iv.0.abs().max(iv.1.abs())
}

/// An i16 expression.
#[derive(Clone)]
enum E {
    Num(i64),
    Var(String),
    Neg(Box<E>),
    Abs(Box<E>),
    Bin(char, Box<E>, Box<E>),
    If(Box<C>, Box<E>, Box<E>),
    Call(String, Vec<E>),
    Index(String, Box<Ix>),
    /// An expression of an enum kit's (`Prog::kits`, by index): its form (`show_enum`'s number), its arguments,
    /// and its constants.
    Enum(usize, u8, Vec<E>, Vec<i64>),
}

/// A kit of enum types and helpers, named by its index k (the type `Ek{k}`, the helpers `ek{k}`, ...).
#[derive(Clone)]
enum Kit {
    /// `enum Ek { A, B, C }`, numbered or not; `ek(v)` is A below t0, B below t1, else C.
    Colour {
        nums: Option<[i64; 3]>,
        t: [i64; 2],
        never: bool,
    },
    /// `enum Ek { Dot, One(i16), Two(i16, i16), Byte(u8), Flag(bool) }`; `ek(a, b)` builds one, by a against the
    /// thresholds (Byte, of a u8 constant, for a from t2 to 255: an i16 cast to a u8 is refused, since it wraps);
    /// `ek_size(s)` matches it.
    Shape { t: [i64; 3], k: i64, never: bool },
    /// `ek(v) -> Option<i16>`: Some(v - t) from t up, else None.
    Opt { t: i64, never: bool },
    /// `ek(v) -> Result<i16, i16>`: Err(v - t) below t, else Ok(v + k); `ek_q(v)`, #[inline(never)], is
    /// `Ok(ek(v)? - k2)`.
    Res { t: i64, k: i64, k2: i64, never: bool },
}

impl Kit {
    /// The first of its kind's form numbers, and how many forms it has.
    fn forms(&self) -> (u8, u8) {
        match self {
            Kit::Colour { .. } => (0, 2),
            Kit::Shape { .. } => (10, 2),
            Kit::Opt { .. } => (20, 5),
            Kit::Res { .. } => (30, 4),
        }
    }

    fn arity(&self) -> usize {
        if matches!(self, Kit::Shape { .. }) {
            2
        } else {
            1
        }
    }
}

/// An index: a usize loop variable as it is, or an i16 expression cast, straight away (`(e) as usize`, which
/// with overflow checks is refused) or from a local first.
#[derive(Clone)]
enum Ix {
    Usize(String),
    Cast(E, bool),
}

/// A condition.
#[derive(Clone)]
enum C {
    Lit(bool),
    Cmp(&'static str, E, E),
    And(Box<C>, Box<C>),
    Or(Box<C>, Box<C>),
    Not(Box<C>),
    Flag(String),
}

/// A statement.
#[derive(Clone)]
enum S {
    /// `let mut v = e;`
    Let(String, E),
    /// `let b = c;`
    Flag(String, C),
    /// `v = e;`, `v += e;`, `v -= e;`
    Set(String, &'static str, E),
    Out(E),
    If(C, Vec<S>, Vec<S>),
    /// `let mut k: i16 = 0; while k < e { body k += 1; }`
    While(String, E, Vec<S>),
    /// `for v in a..b { body }`, over i16; or over usize when the bound is an array's length.
    For(String, E, E, bool, Vec<S>),
    /// `if c { break; }`
    Break(C),
    /// `let mut a: [i16; N] = [...];`
    Array(String, Vec<i64>),
    /// `a[i] = e;`
    Store(String, Ix, E),
    /// `bump(&mut v, e);`
    Bump(String, String, E),
    /// `unsafe { assume(c) };`, of the inputs, which the inputs keep
    Assume(C),
}

#[derive(Clone)]
struct Func {
    name: String,
    /// The first a `&mut i16` (and nothing is returned) when `by_ref`.
    params: Vec<String>,
    by_ref: bool,
    never: bool,
    body: Vec<S>,
    ret: Option<E>,
}

/// A generated program and the ranges its inputs are drawn from.
#[derive(Clone)]
pub struct Prog {
    kits: Vec<Kit>,
    consts: Vec<(String, i64)>,
    tables: Vec<(String, Vec<i64>)>,
    funcs: Vec<Func>,
    inputs: Vec<Iv>,
    main: Vec<S>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// An i16 that is read but not set (an input, an i16 loop variable).
    Int,
    /// An i16 local that is set too.
    Mut,
    /// A while loop's counter: read, never set but by the loop.
    Counter,
    /// A usize loop variable that runs to an array's length.
    Idx(usize),
    Flag,
    /// A local array of this length.
    Array(usize),
}

struct Var {
    name: String,
    kind: Kind,
    iv: Iv,
}

/// What a helper gives back, for the ranges of its callers.
struct Sig {
    name: String,
    arity: usize,
    by_ref: bool,
    ret: Iv,
}

struct Gen {
    rng: Rng,
    vars: Vec<Var>,
    /// Expressions made so far, to be used again, each with its range and how many variables were in scope when
    /// it was made (it uses only those).
    pool: Vec<(E, Iv, usize)>,
    consts: Vec<(String, i64)>,
    tables: Vec<(String, Vec<i64>)>,
    sigs: Vec<Sig>,
    kits: Vec<Kit>,
    /// The variables read somewhere.
    used: HashSet<String>,
    fresh: usize,
    loops: usize,
    nest: usize,
    /// Statements still to make in this program.
    budget: i64,
}

const EDGES: [i64; 14] = [0, 1, -1, 2, 999, -999, 998, -998, 99, 100, -100, 500, 10, -10];
const NUMBERS: [i64; 20] = [
    0, 1, -1, 2, 3, 5, 7, 9, 10, 11, 12, 99, 100, 101, 500, 998, 999, -999, -998, -100,
];
const DIVISORS: [i64; 16] = [1, 2, 3, 4, 5, 7, 9, 10, -1, -3, -10, 100, 999, 13, 6, 0];
const FACTORS: [i64; 10] = [2, 3, 5, 10, -1, 11, 4, 9, 0, -2];
const INPUTS: [Iv; 9] = [
    (-999, 999),
    (-99, 99),
    (0, 9),
    (1, 999),
    (-9, 9),
    (0, 999),
    (-5, 20),
    (1, 12),
    (-999, 0),
];

impl Gen {
    fn chance(&mut self, pct: u64) -> bool {
        self.rng.next() % 100 < pct
    }

    fn below(&mut self, n: usize) -> usize {
        (self.rng.next() % n as u64) as usize
    }

    fn pick<T: Clone>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())].clone()
    }

    fn name(&mut self, stem: &str) -> String {
        self.fresh += 1;
        format!("{stem}{}", self.fresh)
    }

    /// Choose among weights: the index of the one chosen.
    fn weighted(&mut self, w: &[u64]) -> usize {
        let total: u64 = w.iter().sum();
        let mut r = self.rng.next() % total.max(1);
        for (i, &x) in w.iter().enumerate() {
            if r < x {
                return i;
            }
            r -= x;
        }
        0
    }

    fn scalars(&self) -> Vec<usize> {
        (0..self.vars.len())
            .filter(|&i| matches!(self.vars[i].kind, Kind::Int | Kind::Mut | Kind::Counter | Kind::Idx(_)))
            .collect()
    }

    fn of_kind(&self, f: impl Fn(Kind) -> bool) -> Vec<usize> {
        (0..self.vars.len()).filter(|&i| f(self.vars[i].kind)).collect()
    }

    fn var_expr(&mut self, i: usize) -> (E, Iv) {
        self.used.insert(self.vars[i].name.clone());
        let v = &self.vars[i];
        match v.kind {
            Kind::Idx(_) => (E::Var(format!("({} as i16)", v.name)), v.iv),
            _ => (E::Var(v.name.clone()), v.iv),
        }
    }

    fn leaf(&mut self) -> (E, Iv) {
        let vars = self.scalars();
        if !vars.is_empty() && self.chance(65) {
            let i = self.pick(&vars);
            return self.var_expr(i);
        }
        if !self.consts.is_empty() && self.chance(25) {
            let (n, v) = self.pick(&self.consts.clone());
            return (E::Var(n), (v, v));
        }
        let v = if self.chance(70) {
            self.pick(&NUMBERS)
        } else {
            self.rng.range(-20, 20)
        };
        (E::Num(v), (v, v))
    }

    /// An expression that is on a card as far as the generator can tell, mostly: else cut down.
    fn expr(&mut self, depth: u32) -> (E, Iv) {
        for _ in 0..4 {
            let (e, iv) = self.expr1(depth);
            if fits(iv) || self.chance(10) {
                return (e, iv);
            }
        }
        let (e, iv) = self.expr1(depth);
        if fits(iv) {
            return (e, iv);
        }
        if self.chance(50) {
            (E::Bin('%', Box::new(e), Box::new(E::Num(1000))), (-999, 999))
        } else {
            (
                E::Bin('/', Box::new(e), Box::new(E::Num(100))),
                (iv.0 / 100, iv.1 / 100),
            )
        }
    }

    fn expr1(&mut self, depth: u32) -> (E, Iv) {
        if depth == 0 || self.chance(25) {
            return self.leaf();
        }
        let arrays = self.of_kind(|k| matches!(k, Kind::Array(_)));
        let calls: Vec<usize> = (0..self.sigs.len()).filter(|&i| !self.sigs[i].by_ref).collect();
        let w = [
            20,                                   // + -
            8,                                    // a chain of them
            8,                                    // * constant
            3,                                    // * variable
            8,                                    // / constant
            6,                                    // % constant
            5,                                    // / variable
            4,                                    // % variable
            4,                                    // -
            if self.chance(30) { 1 } else { 0 },  // abs
            8,                                    // if
            if calls.is_empty() { 0 } else { 8 }, // call
            if self.tables.is_empty() && arrays.is_empty() {
                0
            } else {
                9
            }, // index
            if self.pool.is_empty() { 0 } else { 8 }, // again
            if self.kits.is_empty() { 0 } else { 10 }, // an enum kit's
        ];
        let d = depth - 1;
        let choice = self.weighted(&w);
        let made = match choice {
            0 => {
                let (a, x) = self.expr(d);
                let (b, y) = self.expr(d);
                if self.chance(50) {
                    (E::Bin('+', Box::new(a), Box::new(b)), (x.0 + y.0, x.1 + y.1))
                } else {
                    (E::Bin('-', Box::new(a), Box::new(b)), (x.0 - y.1, x.1 - y.0))
                }
            }
            1 => {
                let (mut e, mut iv) = self.leaf();
                for _ in 0..2 + self.below(3) {
                    let (b, y) = if self.chance(25) {
                        let (l, z) = self.leaf();
                        let k = self.pick(&[2, 3, -1, 10]);
                        (E::Bin('*', Box::new(l), Box::new(E::Num(k))), mul(z, (k, k)))
                    } else {
                        self.leaf()
                    };
                    if self.chance(60) {
                        e = E::Bin('+', Box::new(e), Box::new(b));
                        iv = (iv.0 + y.0, iv.1 + y.1);
                    } else {
                        e = E::Bin('-', Box::new(e), Box::new(b));
                        iv = (iv.0 - y.1, iv.1 - y.0);
                    }
                }
                (e, iv)
            }
            2 => {
                let (a, x) = self.expr(d);
                let k = self.pick(&FACTORS);
                (E::Bin('*', Box::new(a), Box::new(E::Num(k))), mul(x, (k, k)))
            }
            3 => {
                let (a, x) = self.expr(d);
                let (b, y) = self.leaf();
                (E::Bin('*', Box::new(a), Box::new(b)), mul(x, y))
            }
            4 | 5 => {
                let (a, x) = self.expr(d);
                let k = self.pick(&DIVISORS);
                let k = if k == 0 && self.chance(90) { 7 } else { k };
                let op = if choice == 4 { '/' } else { '%' };
                let iv = match (k, op) {
                    (0, _) => x,
                    (_, '/') => div(x, k),
                    _ => rem(x, k.abs()),
                };
                (E::Bin(op, Box::new(a), Box::new(E::Num(k))), iv)
            }
            6 | 7 => {
                let (a, x) = self.expr(d);
                let (b, y) = if self.chance(70) { self.leaf() } else { self.expr(d) };
                if self.chance(55) {
                    (E::Bin('/', Box::new(a), Box::new(b)), (-mag(x), mag(x)))
                } else {
                    let m = mag(x).min((mag(y) - 1).max(0));
                    (E::Bin('%', Box::new(a), Box::new(b)), (-m, m))
                }
            }
            8 => {
                let (a, x) = self.expr(d);
                (E::Neg(Box::new(a)), (-x.1, -x.0))
            }
            9 => {
                let (a, x) = self.expr(d);
                let lo = if x.0 <= 0 && x.1 >= 0 {
                    0
                } else {
                    x.0.abs().min(x.1.abs())
                };
                (E::Abs(Box::new(a)), (lo, mag(x)))
            }
            10 => {
                let c = self.cond(d);
                let (a, x) = self.expr(d);
                // the same value both ways, for jump threading
                if self.chance(25) {
                    (E::If(Box::new(c), Box::new(a.clone()), Box::new(a)), x)
                } else {
                    let (b, y) = self.expr(d);
                    (E::If(Box::new(c), Box::new(a), Box::new(b)), join(x, y))
                }
            }
            11 => {
                let i = self.pick(&calls);
                let (name, arity, ret) = (self.sigs[i].name.clone(), self.sigs[i].arity, self.sigs[i].ret);
                let args = (0..arity).map(|_| self.expr(d.min(1)).0).collect();
                (E::Call(name, args), ret)
            }
            12 => self.index(d),
            13 => {
                let k = self.below(self.pool.len());
                let (e, iv, _) = self.pool[k].clone();
                return (e, iv);
            }
            _ => self.enum_expr(d),
        };
        self.remember(made)
    }

    /// An expression of one of the program's enum kits, and its range.
    fn enum_expr(&mut self, d: u32) -> (E, Iv) {
        let i = self.below(self.kits.len());
        let kit = self.kits[i].clone();
        let (first, n) = kit.forms();
        let form = self.below(n as usize) as u8;
        let mut args = vec![];
        let mut ivs = vec![];
        for _ in 0..kit.arity() {
            let (e, iv) = self.expr(d.min(1));
            args.push(e);
            ivs.push(iv);
        }
        let ks: Vec<i64> = (0..3).map(|_| self.rng.range(-20, 20)).collect();
        let x = ivs[0];
        let (k, k2) = ((ks[0], ks[0]), (ks[1], ks[1]));
        let add = |a: Iv, b: Iv| (a.0 + b.0, a.1 + b.1);
        let sub = |a: Iv, b: Iv| (a.0 - b.1, a.1 - b.0);
        let iv = match &kit {
            Kit::Colour { nums, .. } => match form {
                0 => {
                    let n = nums.unwrap_or([0, 1, 2]);
                    (*n.iter().min().unwrap(), *n.iter().max().unwrap())
                }
                _ => (ks[0].min(ks[1]).min(ks[2]), ks[0].max(ks[1]).max(ks[2])),
            },
            Kit::Shape { k: sk, .. } => {
                let y = ivs[1];
                let one = sub(y, x);
                match form {
                    0 => join(join(join((*sk, *sk), add(one, (*sk, *sk))), sub(x, y)), (-1, 255)),
                    _ => join(join(add(x, y), one), k),
                }
            }
            Kit::Opt { t, .. } => {
                let some = sub(x, (*t, *t));
                match form {
                    0 => join(some, k),
                    1 | 4 => join(add(some, k), k2),
                    2 => join(sub(some, k), k2),
                    _ => join(k, k2),
                }
            }
            Kit::Res { t, k: rk, k2: rk2, .. } => {
                let ok = sub(add(x, (*rk, *rk)), (*rk2, *rk2));
                let err = sub(x, (*t, *t));
                match form {
                    0 => join(ok, add(err, k)),
                    1 | 3 => join(ok, k),
                    _ => join(k, k2),
                }
            }
        };
        (E::Enum(i, first + form, args, ks), iv)
    }

    fn remember(&mut self, made: (E, Iv)) -> (E, Iv) {
        let made = (made.0, cap(made.1));
        if self.chance(40) {
            self.pool.push((made.0.clone(), made.1, self.vars.len()));
        }
        made
    }

    /// An index into an array of this length: mostly in bounds as far as the generator can tell, sometimes not.
    fn subscript(&mut self, len: usize, d: u32) -> Ix {
        let idx: Vec<usize> = self.of_kind(|k| matches!(k, Kind::Idx(n) if n <= len));
        if !idx.is_empty() && self.chance(60) {
            let i = self.pick(&idx);
            self.used.insert(self.vars[i].name.clone());
            return Ix::Usize(self.vars[i].name.clone());
        }
        let direct = self.chance(20);
        let inside: Vec<usize> = self
            .scalars()
            .into_iter()
            .filter(|&i| !matches!(self.vars[i].kind, Kind::Idx(_)))
            .filter(|&i| self.vars[i].iv.0 >= 0 && self.vars[i].iv.1 < len as i64)
            .collect();
        if !inside.is_empty() && self.chance(60) {
            let i = self.pick(&inside);
            self.used.insert(self.vars[i].name.clone());
            return Ix::Cast(E::Var(self.vars[i].name.clone()), direct);
        }
        let (e, iv) = self.expr(d.min(1));
        let n = || Box::new(E::Num(len as i64));
        let bin = |op, a: E, b: Box<E>| E::Bin(op, Box::new(a), b);
        match self.below(5) {
            0 | 1 if iv.0 >= 0 => Ix::Cast(bin('%', e, n()), direct),
            // (e % n + n) % n, in bounds whatever the sign
            0 | 1 => Ix::Cast(bin('%', bin('+', bin('%', e, n()), n()), n()), direct),
            2 if self.chance(5) => Ix::Cast(bin('%', E::Abs(Box::new(e)), n()), direct),
            // a constant index: out of bounds now and then, which rustc sees
            3 => {
                let past = if self.chance(10) { 0 } else { 1 };
                Ix::Cast(E::Num(self.rng.range(0, len as i64 - past)), direct)
            }
            _ => Ix::Cast(e, direct),
        }
    }

    fn index(&mut self, d: u32) -> (E, Iv) {
        let arrays = self.of_kind(|k| matches!(k, Kind::Array(_)));
        if !arrays.is_empty() && (self.tables.is_empty() || self.chance(50)) {
            let i = self.pick(&arrays);
            let Kind::Array(len) = self.vars[i].kind else {
                unreachable!()
            };
            let (name, iv) = (self.vars[i].name.clone(), self.vars[i].iv);
            let ix = self.subscript(len, d);
            return (E::Index(name, Box::new(ix)), iv);
        }
        let (name, t) = self.pick(&self.tables.clone());
        let iv = (*t.iter().min().unwrap(), *t.iter().max().unwrap());
        let ix = self.subscript(t.len(), d);
        (E::Index(name, Box::new(ix)), iv)
    }

    fn cond(&mut self, depth: u32) -> C {
        let flags = self.of_kind(|k| k == Kind::Flag);
        let w = [
            50,
            25,
            if flags.is_empty() { 0 } else { 8 },
            if depth > 0 { 10 } else { 0 },
            if depth > 0 { 5 } else { 0 },
        ];
        let op = self.pick(&["<", "<=", ">", ">=", "==", "!="]);
        match self.weighted(&w) {
            0 => {
                let (e, iv) = self.expr(depth);
                // an edge, or an end of the value's own range
                let k = if self.chance(30) && iv.0 > -2000 && iv.1 < 2000 {
                    self.pick(&[iv.0, iv.1, iv.0 + 1, iv.1 - 1]).clamp(-999, 999)
                } else {
                    self.pick(&EDGES)
                };
                C::Cmp(op, e, E::Num(k))
            }
            1 => C::Cmp(op, self.expr(depth).0, self.expr(depth).0),
            2 => {
                let i = self.pick(&flags);
                C::Flag(self.vars[i].name.clone())
            }
            3 => {
                let (a, b) = (self.cond(depth - 1), self.cond(depth - 1));
                if self.chance(50) {
                    C::And(Box::new(a), Box::new(b))
                } else {
                    C::Or(Box::new(a), Box::new(b))
                }
            }
            _ => C::Not(Box::new(self.cond(depth - 1))),
        }
    }

    /// Up to n statements, fewer when the program's budget runs out: but at least one, a simple one, for a body
    /// that is better not empty (n > 0).
    fn block(&mut self, n: usize) -> Vec<S> {
        let mut out = vec![];
        for k in 0..n {
            if self.budget <= 0 && k > 0 {
                break;
            }
            self.budget -= 1;
            out.extend(self.stmt());
        }
        out
    }

    /// Run `f` in a scope of its own: the variables it declares, and the expressions made of them, end with it.
    fn scoped(&mut self, f: impl FnOnce(&mut Gen) -> Vec<S>) -> Vec<S> {
        let n = self.vars.len();
        let mut body = f(self);
        self.print_unused(n, &mut body);
        self.vars.truncate(n);
        self.pool.retain(|p| p.2 <= n);
        body
    }

    /// Print most of the locals from `from` on that nothing read: a value no one reads is code the passes delete,
    /// and few programs are written so.
    fn print_unused(&mut self, from: usize, body: &mut Vec<S>) {
        for i in from..self.vars.len() {
            if self.vars[i].kind == Kind::Mut && !self.used.contains(&self.vars[i].name) && self.chance(85) {
                body.push(S::Out(E::Var(self.vars[i].name.clone())));
            }
        }
    }

    fn ranges(&self) -> Vec<Iv> {
        self.vars.iter().map(|v| v.iv).collect()
    }

    fn stmt(&mut self) -> Vec<S> {
        let muts = self.of_kind(|k| k == Kind::Mut);
        let arrays = self.of_kind(|k| matches!(k, Kind::Array(_)));
        let bumps: Vec<usize> = (0..self.sigs.len()).filter(|&i| self.sigs[i].by_ref).collect();
        let inner = self.nest < 3 && self.budget > 0;
        let w = [
            20,                                          // let
            if muts.is_empty() { 0 } else { 16 },        // set
            12,                                          // out
            if inner { 10 } else { 0 },                  // if
            if inner && self.loops < 2 { 6 } else { 0 }, // while
            if inner && self.loops < 2 { 5 } else { 0 }, // for over i16
            if inner && self.loops < 2 && (!arrays.is_empty() || !self.tables.is_empty()) {
                5
            } else {
                0
            },
            if self.loops > 0 { 7 } else { 0 },                      // break
            if arrays.len() < 2 { 4 } else { 0 },                    // array
            if arrays.is_empty() { 0 } else { 7 },                   // store
            if bumps.is_empty() || muts.is_empty() { 0 } else { 5 }, // bump
            3,                                                       // flag
        ];
        match self.weighted(&w) {
            0 => {
                let (e, iv) = self.expr(2);
                let v = self.name("v");
                self.vars.push(Var {
                    name: v.clone(),
                    kind: Kind::Mut,
                    iv,
                });
                vec![S::Let(v, e)]
            }
            1 => {
                let i = self.pick(&muts);
                let (e, iv) = self.expr(2);
                let old = self.vars[i].iv;
                let (op, new) = match self.below(3) {
                    0 => ("=", iv),
                    1 => ("+=", (old.0 + iv.0, old.1 + iv.1)),
                    _ => ("-=", (old.0 - iv.1, old.1 - iv.0)),
                };
                self.vars[i].iv = cap(new);
                vec![S::Set(self.vars[i].name.clone(), op, e)]
            }
            2 => vec![S::Out(self.expr(2).0)],
            3 => {
                let c = self.cond(1);
                let before = self.ranges();
                self.nest += 1;
                let then = self.scoped(|g| {
                    let n = 1 + g.below(3);
                    g.block(n)
                });
                let after = self.ranges();
                for (v, iv) in self.vars.iter_mut().zip(&before) {
                    v.iv = *iv;
                }
                let other = if self.chance(12) {
                    then.clone()
                } else {
                    self.scoped(|g| {
                        let n = g.below(3);
                        g.block(n)
                    })
                };
                self.nest -= 1;
                for (v, iv) in self.vars.iter_mut().zip(&after) {
                    v.iv = join(v.iv, *iv);
                }
                vec![S::If(c, then, other)]
            }
            4 => {
                let (bound, n) = self.trip();
                let k = self.name("k");
                let body = self.looped(n, |g| {
                    g.vars.push(Var {
                        name: k.clone(),
                        kind: Kind::Counter,
                        iv: (0, n.max(1) - 1),
                    });
                    let m = 1 + g.below(3);
                    g.block(m)
                });
                vec![S::While(k, bound, body)]
            }
            5 => {
                let lo = self.pick(&[0, 0, 1, -2, -1]);
                let (hi, n) = self.trip();
                let i = self.name("i");
                let body = self.looped(n, |g| {
                    g.vars.push(Var {
                        name: i.clone(),
                        kind: Kind::Int,
                        iv: (lo, (lo + n).max(lo)),
                    });
                    let m = 1 + g.below(3);
                    g.block(m)
                });
                vec![S::For(i, E::Num(lo), hi, false, body)]
            }
            6 => {
                // to an array's length, or one past it (a bounds check that fails at the end)
                let mut lens: Vec<usize> = self.tables.iter().map(|t| t.1.len()).collect();
                for &a in &arrays {
                    if let Kind::Array(n) = self.vars[a].kind {
                        lens.push(n);
                    }
                }
                let len = self.pick(&lens) as i64 + if self.chance(10) { 1 } else { 0 };
                let i = self.name("j");
                let body = self.looped(len, |g| {
                    g.vars.push(Var {
                        name: i.clone(),
                        kind: Kind::Idx(len as usize),
                        iv: (0, len - 1),
                    });
                    let m = 1 + g.below(3);
                    g.block(m)
                });
                vec![S::For(i, E::Num(0), E::Num(len), true, body)]
            }
            7 => vec![S::Break(self.cond(1))],
            8 => {
                let n = 2 + self.below(4);
                let a = self.name("a");
                let vals: Vec<i64> = (0..n).map(|_| self.pick(&NUMBERS[..14])).collect();
                let iv = (*vals.iter().min().unwrap(), *vals.iter().max().unwrap());
                self.vars.push(Var {
                    name: a.clone(),
                    kind: Kind::Array(n),
                    iv,
                });
                vec![S::Array(a, vals)]
            }
            9 => {
                let a = self.pick(&arrays);
                let Kind::Array(len) = self.vars[a].kind else {
                    unreachable!()
                };
                let ix = self.subscript(len, 1);
                let (e, iv) = self.expr(2);
                self.vars[a].iv = cap(join(self.vars[a].iv, iv));
                vec![S::Store(self.vars[a].name.clone(), ix, e)]
            }
            10 => {
                let i = self.pick(&muts);
                let f = self.pick(&bumps);
                let (mut e, _) = self.expr(1);
                // the value can't read the variable lent to the call
                if uses(&e, &self.vars[i].name) {
                    e = E::Num(self.pick(&NUMBERS));
                }
                let r = self.sigs[f].ret;
                self.vars[i].iv = cap((self.vars[i].iv.0 + r.0, self.vars[i].iv.1 + r.1));
                vec![S::Bump(self.sigs[f].name.clone(), self.vars[i].name.clone(), e)]
            }
            _ => {
                let c = self.cond(1);
                let b = self.name("b");
                self.vars.push(Var {
                    name: b.clone(),
                    kind: Kind::Flag,
                    iv: (0, 1),
                });
                vec![S::Flag(b, c)]
            }
        }
    }

    /// A loop's bound: a constant, or a variable whose range is small, and how many times round at most.
    fn trip(&mut self) -> (E, i64) {
        let small: Vec<usize> = self
            .scalars()
            .into_iter()
            .filter(|&i| !matches!(self.vars[i].kind, Kind::Idx(_)) && self.vars[i].iv.1 <= 30)
            .collect();
        if !small.is_empty() && self.chance(40) {
            let i = self.pick(&small);
            let (e, iv) = self.var_expr(i);
            return (e, iv.1.max(0));
        }
        let n = self.pick(&[1, 2, 3, 4, 5, 8, 10]);
        (E::Num(n), n)
    }

    /// A loop's body, n times round at most: the variables it changes are widened as if it went round n times.
    fn looped(&mut self, n: i64, f: impl FnOnce(&mut Gen) -> Vec<S>) -> Vec<S> {
        let before = self.ranges();
        self.loops += 1;
        self.nest += 1;
        let body = self.scoped(f);
        self.loops -= 1;
        self.nest -= 1;
        let n = n.clamp(1, 1000);
        for (v, b) in self.vars.iter_mut().zip(&before) {
            let lo = if v.iv.0 < b.0 { b.0 + n * (v.iv.0 - b.0) } else { b.0 };
            let hi = if v.iv.1 > b.1 { b.1 + n * (v.iv.1 - b.1) } else { b.1 };
            v.iv = cap((lo.min(v.iv.0), hi.max(v.iv.1)));
        }
        body
    }

    fn func(&mut self, k: usize) -> Func {
        let by_ref = self.chance(25);
        let name = if by_ref { format!("bump{k}") } else { format!("f{k}") };
        let arity = if by_ref { 2 } else { 1 + self.below(2) };
        let params: Vec<String> = if by_ref {
            vec!["v".into(), "d".into()]
        } else {
            ["p", "q"][..arity].iter().map(|s| s.to_string()).collect()
        };
        let never = self.chance(50);
        self.vars.clear();
        self.pool.clear();
        if by_ref {
            self.vars.push(Var {
                name: "*v".into(),
                kind: Kind::Mut,
                iv: (-300, 300),
            });
            self.vars.push(Var {
                name: "d".into(),
                kind: Kind::Int,
                iv: (-300, 300),
            });
        } else {
            for p in &params {
                self.vars.push(Var {
                    name: p.clone(),
                    kind: Kind::Int,
                    iv: (-300, 300),
                });
            }
        }
        self.budget = self.below(3) as i64;
        let n = self.budget as usize;
        let mut body = self.block(n);
        let from = if by_ref { 2 } else { arity };
        self.print_unused(from, &mut body);
        let (ret, iv) = if by_ref {
            let (e, _) = self.expr(2);
            body.push(S::Set("*v".into(), self.pick(&["=", "+=", "-="]), e));
            (None, (self.vars[0].iv.0 - 300, self.vars[0].iv.1 + 300))
        } else {
            let (e, iv) = self.expr(2);
            (Some(e), iv)
        };
        self.sigs.push(Sig {
            name: name.clone(),
            arity,
            by_ref,
            ret: cap(iv),
        });
        Func {
            name,
            params,
            by_ref,
            never,
            body,
            ret,
        }
    }
}

/// Whether an expression reads this variable.
fn uses(e: &E, v: &str) -> bool {
    let c = |c: &C| match c {
        C::Cmp(_, a, b) => uses(a, v) || uses(b, v),
        C::Lit(_) => false,
        C::Flag(_) | C::And(..) | C::Or(..) | C::Not(_) => true,
    };
    match e {
        E::Num(_) => false,
        E::Var(n) => n == v,
        E::Neg(a) | E::Abs(a) => uses(a, v),
        E::Bin(_, a, b) => uses(a, v) || uses(b, v),
        E::If(k, a, b) => c(k) || uses(a, v) || uses(b, v),
        E::Call(_, args) => args.iter().any(|a| uses(a, v)),
        E::Index(_, ix) => matches!(&**ix, Ix::Cast(i, _) if uses(i, v)),
        E::Enum(_, _, args, _) => args.iter().any(|a| uses(a, v)),
    }
}

fn mul(a: Iv, b: Iv) -> Iv {
    let p = [a.0 * b.0, a.0 * b.1, a.1 * b.0, a.1 * b.1];
    cap((*p.iter().min().unwrap(), *p.iter().max().unwrap()))
}

fn div(a: Iv, k: i64) -> Iv {
    let (x, y) = (a.0 / k, a.1 / k);
    (x.min(y), x.max(y))
}

fn rem(a: Iv, k: i64) -> Iv {
    let m = (k - 1).max(0);
    (
        if a.0 < 0 { -m.min(-a.0) } else { 0 },
        if a.1 > 0 { m.min(a.1) } else { 0 },
    )
}

/// Program `n` of a seed; `enums` percent of programs have enum kits.
pub fn program(seed: u64, n: u64, enums: u64) -> Prog {
    let kits = kits(seed, n, enums);
    let mut g = Gen {
        rng: Rng::new(seed, "generated", n),
        vars: vec![],
        pool: vec![],
        consts: vec![],
        tables: vec![],
        sigs: vec![],
        kits: kits.clone(),
        used: HashSet::new(),
        fresh: 0,
        loops: 0,
        nest: 0,
        budget: 0,
    };
    for k in 0..g.below(3) {
        let v = g.pick(&NUMBERS);
        g.consts.push((format!("K{k}"), v));
    }
    for k in 0..g.below(3) {
        let n = 2 + g.below(6);
        let t: Vec<i64> = (0..n)
            .map(|_| {
                if g.chance(50) {
                    g.pick(&NUMBERS)
                } else {
                    g.rng.range(-50, 50)
                }
            })
            .collect();
        g.tables.push((format!("T{k}"), t));
    }
    let funcs: Vec<Func> = (0..g.weighted(&[3, 4, 2])).map(|k| g.func(k)).collect();
    g.vars.clear();
    g.pool.clear();
    let mut main = vec![];
    let inputs: Vec<Iv> = (0..1 + g.below(3)).map(|_| g.pick(&INPUTS)).collect();
    for (i, &iv) in inputs.iter().enumerate() {
        let x = format!("x{i}");
        main.push(S::Let(x.clone(), E::Call("inp".into(), vec![])));
        g.vars.push(Var {
            name: x.clone(),
            kind: Kind::Int,
            iv,
        });
        // a promise the inputs keep: a bound of the range, or that 0 is out of it
        if fits((iv.0 - 1, iv.1 + 1)) || g.chance(30) {
            let mut promises = vec![];
            if iv.0 > -999 && g.chance(70) {
                promises.push(C::Cmp(g.pick(&[">=", ">"]), E::Var(x.clone()), E::Num(iv.0)));
                if let C::Cmp(">", _, E::Num(k)) = promises.last_mut().unwrap() {
                    *k -= 1;
                }
            }
            if iv.1 < 999 && g.chance(70) {
                promises.push(C::Cmp("<=", E::Var(x.clone()), E::Num(iv.1)));
            }
            if (iv.0 > 0 || iv.1 < 0) && g.chance(30) {
                promises.push(C::Cmp("!=", E::Var(x.clone()), E::Num(0)));
            }
            if let Some(first) = promises.pop() {
                let c = promises
                    .into_iter()
                    .fold(first, |a, b| C::And(Box::new(b), Box::new(a)));
                main.push(S::Assume(c));
            }
        }
    }
    g.budget = 2 + g.below(6) as i64;
    let n = g.budget as usize;
    let from = g.vars.len();
    main.extend(g.block(n));
    g.print_unused(from, &mut main);
    let (e, _) = g.expr(2);
    main.push(S::Out(e));
    Prog {
        kits,
        consts: g.consts,
        tables: g.tables,
        funcs,
        inputs,
        main,
    }
}

/// The enum kits of program `n` of a seed, from a stream of their own: a program without any is as it was before
/// there were kits.
fn kits(seed: u64, n: u64, enums: u64) -> Vec<Kit> {
    let mut r = Rng::new(seed, "enums", n);
    let mut kits = vec![];
    if r.next() % 100 >= enums {
        return kits;
    }
    for _ in 0..1 + r.next() % 2 {
        let never = r.next().is_multiple_of(2);
        kits.push(match r.next() % 4 {
            0 => {
                let nums = r.next().is_multiple_of(2).then(|| {
                    let a = r.range(-20, 20);
                    [a, a + 1 + r.range(0, 30), a - 1 - r.range(0, 30)]
                });
                let t0 = r.range(-50, 50);
                Kit::Colour {
                    nums,
                    t: [t0, t0 + r.range(0, 60)],
                    never,
                }
            }
            1 => {
                let t0 = r.range(-60, 0);
                let t1 = t0 + r.range(0, 60);
                Kit::Shape {
                    t: [t0, t1, t1.max(0) + r.range(0, 60)],
                    k: r.range(-20, 20),
                    never,
                }
            }
            2 => Kit::Opt {
                t: r.range(-30, 30),
                never,
            },
            _ => Kit::Res {
                t: r.range(-30, 30),
                k: r.range(-20, 20),
                k2: r.range(-20, 20),
                never,
            },
        });
    }
    kits
}

// The program as Rust.

fn show_e(e: &E) -> String {
    match e {
        E::Num(v) if *v < 0 => format!("({v})"),
        E::Num(v) => v.to_string(),
        E::Var(v) => v.clone(),
        E::Neg(a) => format!("-{}", show_e(a)),
        E::Abs(a) => format!("i16::abs({})", show_e(a)),
        E::Bin(op, a, b) => format!("({} {op} {})", show_e(a), show_e(b)),
        E::If(c, a, b) => format!("(if {} {{ {} }} else {{ {} }})", show_c(c), show_e(a), show_e(b)),
        E::Call(f, args) => format!("{f}({})", args.iter().map(show_e).collect::<Vec<_>>().join(", ")),
        E::Index(a, ix) => format!("{a}[{}]", show_ix(ix)),
        E::Enum(k, form, args, ks) => show_enum(*k, *form, args, ks),
    }
}

/// An enum kit's expression as Rust, by its form's number (`Kit::forms`).
fn show_enum(k: usize, form: u8, args: &[E], ks: &[i64]) -> String {
    let a: Vec<String> = args.iter().map(show_e).collect();
    let n = |v: i64| if v < 0 { format!("({v})") } else { v.to_string() };
    let (k0, k1, k2) = (n(ks[0]), n(ks[1]), n(ks[2]));
    let x = &a[0];
    match form {
        // a C-like enum
        0 => format!("(ek{k}({x}) as i16)"),
        1 => format!("(match ek{k}({x}) {{ Ek{k}::A => {k0}, Ek{k}::B => {k1}, Ek{k}::C => {k2} }})"),
        // an enum with data
        10 => format!("ek{k}_size(ek{k}({x}, {}))", a[1]),
        11 => format!(
            "(match ek{k}({x}, {}) {{ Ek{k}::Two(ea, eb) => ea + eb, Ek{k}::One(ea) => ea, _ => {k0} }})",
            a[1]
        ),
        // Option
        20 => format!("ek{k}({x}).unwrap_or({k0})"),
        21 => format!("(match ek{k}({x}) {{ Some(ea) => ea + {k0}, None => {k1} }})"),
        22 => format!("(if let Some(ea) = ek{k}({x}) {{ ea - {k0} }} else {{ {k1} }})"),
        23 => format!("(if ek{k}({x}).is_some() {{ {k0} }} else {{ {k1} }})"),
        24 => format!("ek{k}({x}).map(|ea| ea + {k0}).unwrap_or({k1})"),
        // Result
        30 => format!("(match ek{k}_q({x}) {{ Ok(ea) => ea, Err(eb) => eb + {k0} }})"),
        31 => format!("ek{k}_q({x}).unwrap_or({k0})"),
        32 => format!("(if ek{k}_q({x}).is_ok() {{ {k0} }} else {{ {k1} }})"),
        _ => format!("ek{k}_q({x}).ok().unwrap_or({k0})"),
    }
}

/// The types and helpers of kit k, as Rust.
fn show_kit(k: usize, kit: &Kit) -> String {
    let n = |v: i64| if v < 0 { format!("({v})") } else { v.to_string() };
    let attr = |never: bool| if never { "#[inline(never)]\n" } else { "" };
    match kit {
        Kit::Colour { nums, t, never } => {
            let vs = match nums {
                Some([a, b, c]) => format!("A = {a}, B = {b}, C = {c}"),
                None => "A, B, C".into(),
            };
            format!(
                "\n#[derive(Clone, Copy)]\nenum Ek{k} {{ {vs} }}\n\n{}fn ek{k}(v: i16) -> Ek{k} {{\n    \
                 if v < {} {{ Ek{k}::A }} else if v < {} {{ Ek{k}::B }} else {{ Ek{k}::C }}\n}}\n",
                attr(*never),
                n(t[0]),
                n(t[1])
            )
        }
        Kit::Shape { t, k: sk, never } => format!(
            "\n#[derive(Clone, Copy)]\nenum Ek{k} {{ Dot, One(i16), Two(i16, i16), Byte(u8), Flag(bool) }}\n\n\
             {a}fn ek{k}(a: i16, b: i16) -> Ek{k} {{\n    if a < {t0} {{ Ek{k}::Dot }} else if a < {t1} {{ Ek{k}::One(b - a) }} \
             else if a < {t2} {{ Ek{k}::Two(a, b) }} else if a <= 255 {{ Ek{k}::Byte(if b > a {{ 200 }} else {{ 7 }}) }} else {{ Ek{k}::Flag(b > a) }}\n}}\n\n\
             {a}fn ek{k}_size(s: Ek{k}) -> i16 {{\n    match s {{\n        Ek{k}::Dot => {sk},\n        Ek{k}::One(x) => x + {sk},\n        \
             Ek{k}::Two(x, y) => x - y,\n        Ek{k}::Byte(u) => u as i16,\n        Ek{k}::Flag(f) => if f {{ 1 }} else {{ -1 }},\n    }}\n}}\n",
            a = attr(*never),
            t0 = n(t[0]),
            t1 = n(t[1]),
            t2 = n(t[2]),
            sk = n(*sk)
        ),
        Kit::Opt { t, never } => format!(
            "\n{}fn ek{k}(v: i16) -> Option<i16> {{\n    if v >= {t} {{ Some(v - {t}) }} else {{ None }}\n}}\n",
            attr(*never),
            t = n(*t)
        ),
        Kit::Res { t, k: rk, k2, never } => format!(
            "\n{}fn ek{k}(v: i16) -> Result<i16, i16> {{\n    if v < {t} {{ Err(v - {t}) }} else {{ Ok(v + {rk}) }}\n}}\n\n\
             #[inline(never)]\nfn ek{k}_q(v: i16) -> Result<i16, i16> {{\n    let a = ek{k}(v)?;\n    Ok(a - {k2})\n}}\n",
            attr(*never),
            t = n(*t),
            rk = n(*rk),
            k2 = n(*k2)
        ),
    }
}

fn show_ix(ix: &Ix) -> String {
    match ix {
        Ix::Usize(i) => i.clone(),
        Ix::Cast(E::Var(v), _) => format!("{v} as usize"),
        // constants alone would be typed usize by the cast: `(-20) as usize` doesn't compile
        Ix::Cast(e, true) if has_var(e) => format!("{} as usize", show_e(e)),
        Ix::Cast(e, _) => format!("{{ let i: i16 = {}; i as usize }}", show_e(e)),
    }
}

/// Whether an expression reads a variable (or calls a function), so that rustc types it as the variable is.
fn has_var(e: &E) -> bool {
    match e {
        E::Num(_) => false,
        E::Var(_) | E::Call(..) | E::Index(..) | E::Enum(..) => true,
        E::Neg(a) | E::Abs(a) => has_var(a),
        E::Bin(_, a, b) | E::If(_, a, b) => has_var(a) || has_var(b),
    }
}

fn show_c(c: &C) -> String {
    match c {
        C::Lit(b) => b.to_string(),
        C::Cmp(op, a, b) => format!("{} {op} {}", show_e(a), show_e(b)),
        C::And(a, b) => format!("({} && {})", show_c(a), show_c(b)),
        C::Or(a, b) => format!("({} || {})", show_c(a), show_c(b)),
        C::Not(a) => format!("!({})", show_c(a)),
        C::Flag(b) => b.clone(),
    }
}

fn show_block(body: &[S], depth: usize, out: &mut String) {
    for s in body {
        show_s(s, depth, out);
    }
}

fn show_s(s: &S, depth: usize, out: &mut String) {
    let pad = "    ".repeat(depth);
    let line = |out: &mut String, t: String| {
        out.push_str(&pad);
        out.push_str(&t);
        out.push('\n');
    };
    match s {
        S::Let(v, E::Call(f, args)) if f == "inp" && args.is_empty() => line(out, format!("let {v} = inp();")),
        S::Let(v, e) => line(out, format!("let mut {v}: i16 = {};", show_e(e))),
        S::Flag(b, c) => line(out, format!("let {b} = {};", show_c(c))),
        S::Set(v, op, e) => line(out, format!("{v} {op} {};", show_e(e))),
        S::Out(e) => line(out, format!("out({});", show_e(e))),
        S::If(c, then, other) => {
            line(out, format!("if {} {{", show_c(c)));
            show_block(then, depth + 1, out);
            if other.is_empty() {
                line(out, "}".into());
            } else {
                line(out, "} else {".into());
                show_block(other, depth + 1, out);
                line(out, "}".into());
            }
        }
        S::While(k, bound, body) => {
            line(out, format!("let mut {k}: i16 = 0;"));
            line(out, format!("while {k} < {} {{", show_e(bound)));
            show_block(body, depth + 1, out);
            line(out, format!("    {k} += 1;"));
            line(out, "}".into());
        }
        S::For(i, lo, hi, idx, body) => {
            if *idx {
                line(out, format!("for {i} in 0..{} {{", show_e(hi)));
            } else {
                line(out, format!("for {i} in {}..{} {{", show_e(lo), show_e(hi)));
            }
            show_block(body, depth + 1, out);
            line(out, "}".into());
        }
        S::Break(c) => line(out, format!("if {} {{ break; }}", show_c(c))),
        S::Array(a, vals) => {
            let vs: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
            line(
                out,
                format!("let mut {a}: [i16; {}] = [{}];", vals.len(), vs.join(", ")),
            );
        }
        S::Store(a, ix, e) => line(out, format!("{a}[{}] = {};", show_ix(ix), show_e(e))),
        S::Bump(f, v, e) => line(out, format!("{f}(&mut {v}, {});", show_e(e))),
        S::Assume(c) => line(out, format!("unsafe {{ assume({}) }};", show_c(c))),
    }
}

impl Prog {
    /// The program's source, headed by where it came from.
    pub fn source(&self, head: &str) -> String {
        let mut s = format!("//! {head}\n");
        s += "#![allow(unused, unused_parens, arithmetic_overflow, unconditional_panic, unreachable_code)]\n";
        s += "use cardiac::*;\n\n";
        for (k, v) in &self.consts {
            s += &format!("const {k}: i16 = {v};\n");
        }
        for (t, vals) in &self.tables {
            let vs: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
            s += &format!("const {t}: [i16; {}] = [{}];\n", vals.len(), vs.join(", "));
        }
        // the kits an expression uses (shrinking can take the last use of one away)
        let mut used = vec![false; self.kits.len()];
        exprs(&mut self.clone(), &mut |n| {
            if let Node::E(E::Enum(k, ..)) = n {
                used[*k] = true;
            }
        });
        for (k, kit) in self.kits.iter().enumerate() {
            if used[k] {
                s += &show_kit(k, kit);
            }
        }
        for f in &self.funcs {
            s += "\n";
            if f.never {
                s += "#[inline(never)]\n";
            }
            if f.by_ref {
                s += &format!("fn {}(v: &mut i16, d: i16) {{\n", f.name);
            } else {
                let ps: Vec<String> = f.params.iter().map(|p| format!("{p}: i16")).collect();
                s += &format!("fn {}({}) -> i16 {{\n", f.name, ps.join(", "));
            }
            show_block(&f.body, 1, &mut s);
            if let Some(r) = &f.ret {
                s += &format!("    {}\n", show_e(r));
            }
            s += "}\n";
        }
        s += "\nfn main() {\n";
        show_block(&self.main, 1, &mut s);
        s += "}\n";
        s
    }

    /// The inputs' ranges as tests/fuzz/targets writes them.
    pub fn input_words(&self) -> String {
        self.inputs
            .iter()
            .map(|(a, b)| format!("{a}..{b}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn target(&self, name: &str) -> Target {
        Target {
            name: name.to_string(),
            source: PathBuf::new(),
            inputs: self
                .inputs
                .iter()
                .map(|&(lo, hi)| Inputs { min: 1, max: 1, lo, hi })
                .collect(),
            exact: false,
        }
    }
}

// The program run by the generator itself: what it prints and whether a value passed a card on the way.

/// How a program ends when the generator runs it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum End {
    /// main returned, or the input ran out (both stop: natively exit 0).
    Done,
    /// A check failed: an index, a division by 0.
    Panic,
    /// A promise its input doesn't keep: the generator's mistake.
    Broken,
    TooLong,
}

/// A run of the program as written: its cards, how it ended, and whether any value it computed passed what a card
/// holds. When none did, CARDIAC must print what Rust prints, in every setting: this tells a miscompile from a run
/// past a card more surely than CARDIAC's own checks can, which see only what the passes leave.
pub struct Sim {
    pub cards: Vec<i64>,
    pub end: End,
    /// a value kept (see `Machine::kept`) passed a card
    pub past: bool,
    /// any value passed a card, kept or only in an expression: with overflow checks on, CARDIAC panics there
    pub past_any: bool,
}

impl Sim {
    /// Whether a run that differs is past a card in a setting: with overflow checks on, any value that passed
    /// one (a checked operation stops there); without them, only one that was kept.
    pub fn past_in(&self, checks: bool) -> bool {
        self.past || (checks && self.past_any)
    }
}

#[derive(Clone)]
enum Val {
    Int(i64),
    Flag(bool),
    Array(Vec<i64>),
}

struct Machine<'a> {
    p: &'a Prog,
    input: &'a [i64],
    read: usize,
    cards: Vec<i64>,
    past: bool,
    past_any: bool,
    steps: u64,
    /// The variables in scope, innermost last.
    env: Vec<(String, Val)>,
}

type Go<T> = Result<T, End>;

impl Machine<'_> {
    /// A value worked out in an expression: it may pass a card there, in the accumulator, and must still come out
    /// as Rust's if it is back on one before anything keeps it.
    fn card(&mut self, v: i64) -> i64 {
        if !(-999..=999).contains(&v) {
            self.past_any = true;
        }
        v
    }

    /// A value kept: in a variable, an array, a parameter or a result, a loop's bounds or an index, or output.
    /// Past a card, a cell keeps its last three digits, and the run is no finding.
    fn kept(&mut self, v: i64) -> i64 {
        if !(-999..=999).contains(&v) {
            self.past = true;
            self.past_any = true;
        }
        v
    }

    fn slot(&mut self, name: &str) -> Option<&mut Val> {
        self.env.iter_mut().rev().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    fn int(&mut self, name: &str) -> i64 {
        // a usize loop variable read as an i16
        let name = name
            .strip_prefix('(')
            .and_then(|n| n.strip_suffix(" as i16)"))
            .unwrap_or(name);
        match self.slot(name) {
            Some(Val::Int(v)) => *v,
            _ => self.p.consts.iter().find(|c| c.0 == name).map(|c| c.1).unwrap_or(0),
        }
    }

    fn e(&mut self, x: &E) -> Go<i64> {
        Ok(match x {
            E::Num(v) => *v,
            E::Var(n) => self.int(n),
            E::Neg(a) => {
                let v = -self.e(a)?;
                self.card(v)
            }
            E::Abs(a) => {
                let v = self.e(a)?.abs();
                self.card(v)
            }
            E::Bin(op, a, b) => {
                let (a, b) = (self.e(a)?, self.e(b)?);
                let v = match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    _ if b == 0 => return Err(End::Panic),
                    '/' => a / b,
                    _ => a % b,
                };
                self.card(v)
            }
            E::If(c, a, b) => {
                if self.c(c)? {
                    self.e(a)?
                } else {
                    self.e(b)?
                }
            }
            E::Call(f, args) if f == "inp" && args.is_empty() => {
                let v = *self.input.get(self.read).ok_or(End::Done)?;
                self.read += 1;
                v
            }
            E::Call(f, args) => {
                let mut vals = vec![];
                for a in args {
                    let v = self.e(a)?;
                    vals.push(self.kept(v));
                }
                let func = self.p.funcs.iter().find(|g| &g.name == f).unwrap();
                let saved = std::mem::take(&mut self.env);
                self.env = func
                    .params
                    .iter()
                    .cloned()
                    .zip(vals.into_iter().map(Val::Int))
                    .collect();
                // the result is in the body's scope: it reads the body's locals
                let r = self.stmts(&func.body).and_then(|_| match &func.ret {
                    Some(r) => self.e(r),
                    None => Ok(0),
                });
                self.env = saved;
                let r = r?;
                self.kept(r)
            }
            E::Index(a, ix) => {
                let i = self.ix(ix)?;
                let arr = match self.slot(a) {
                    Some(Val::Array(t)) => t.clone(),
                    _ => self
                        .p
                        .tables
                        .iter()
                        .find(|t| &t.0 == a)
                        .map(|t| t.1.clone())
                        .unwrap_or_default(),
                };
                *arr.get(usize::try_from(i).map_err(|_| End::Panic)?).ok_or(End::Panic)?
            }
            E::Enum(k, form, args, ks) => {
                let mut vals = vec![];
                for a in args {
                    let v = self.e(a)?;
                    vals.push(self.kept(v));
                }
                self.kit(&self.p.kits[*k], *form, &vals, ks)
            }
        })
    }

    /// An enum kit's expression, as `show_enum` writes it and Rust runs it.
    fn kit(&mut self, kit: &Kit, form: u8, a: &[i64], ks: &[i64]) -> i64 {
        let x = a[0];
        match kit {
            Kit::Colour { nums, t, .. } => {
                let v = if x < t[0] {
                    0
                } else if x < t[1] {
                    1
                } else {
                    2
                };
                if form == 0 {
                    nums.unwrap_or([0, 1, 2])[v]
                } else {
                    ks[v]
                }
            }
            Kit::Shape { t, k, .. } => {
                let y = a[1];
                // (the variant, its fields)
                let (variant, f0, f1) = if x < t[0] {
                    (0, 0, 0)
                } else if x < t[1] {
                    let d = self.card(y - x);
                    (1, d, 0)
                } else if x < t[2] {
                    (2, x, y)
                } else if x <= 255 {
                    (3, if y > x { 200 } else { 7 }, 0)
                } else {
                    (4, (y > x) as i64, 0)
                };
                if form == 10 {
                    match variant {
                        0 => *k,
                        1 => self.card(f0 + k),
                        2 => self.card(f0 - f1),
                        3 => f0,
                        _ => {
                            if f0 == 1 {
                                1
                            } else {
                                -1
                            }
                        }
                    }
                } else {
                    match variant {
                        2 => self.card(f0 + f1),
                        1 => f0,
                        _ => ks[0],
                    }
                }
            }
            Kit::Opt { t, .. } => {
                let some = (x >= *t).then(|| self.card(x - t));
                match (form, some) {
                    (20, Some(v)) => v,
                    (21 | 24, Some(v)) => self.card(v + ks[0]),
                    (22, Some(v)) => self.card(v - ks[0]),
                    (23, Some(_)) => ks[0],
                    (20, None) => ks[0],
                    _ => ks[1],
                }
            }
            Kit::Res { t, k, k2, .. } => {
                let r = if x < *t {
                    Err(self.card(x - t))
                } else {
                    let v = self.card(x + k);
                    Ok(self.card(v - k2))
                };
                match (form, r) {
                    (30, Ok(v)) => v,
                    (30, Err(e)) => self.card(e + ks[0]),
                    (31 | 33, Ok(v)) => v,
                    (32, Ok(_)) => ks[0],
                    (32, Err(_)) => ks[1],
                    _ => ks[0],
                }
            }
        }
    }

    fn ix(&mut self, ix: &Ix) -> Go<i64> {
        match ix {
            Ix::Usize(n) => Ok(self.int(n)),
            Ix::Cast(e, _) => {
                let v = self.e(e)?;
                Ok(self.kept(v))
            }
        }
    }

    fn c(&mut self, x: &C) -> Go<bool> {
        Ok(match x {
            C::Lit(b) => *b,
            C::Cmp(op, a, b) => {
                let (a, b) = (self.e(a)?, self.e(b)?);
                match *op {
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    ">=" => a >= b,
                    "==" => a == b,
                    _ => a != b,
                }
            }
            C::And(a, b) => self.c(a)? && self.c(b)?,
            C::Or(a, b) => self.c(a)? || self.c(b)?,
            C::Not(a) => !self.c(a)?,
            C::Flag(n) => matches!(self.slot(n), Some(Val::Flag(true))),
        })
    }

    fn tick(&mut self) -> Go<()> {
        self.steps += 1;
        if self.steps > 1_000_000 {
            return Err(End::TooLong);
        }
        Ok(())
    }

    /// Run statements in a scope of their own; Ok(true) if they broke out of the loop they're in.
    fn block(&mut self, body: &[S]) -> Go<bool> {
        let n = self.env.len();
        let r = self.stmts(body);
        self.env.truncate(n);
        r
    }

    fn stmts(&mut self, body: &[S]) -> Go<bool> {
        for s in body {
            self.tick()?;
            match s {
                S::Let(v, e) => {
                    let x = self.e(e)?;
                    let x = self.kept(x);
                    self.env.push((v.clone(), Val::Int(x)));
                }
                S::Flag(b, c) => {
                    let x = self.c(c)?;
                    self.env.push((b.clone(), Val::Flag(x)));
                }
                S::Set(v, op, e) => {
                    let x = self.e(e)?;
                    let old = self.int(v);
                    let new = match *op {
                        "=" => x,
                        "+=" => old + x,
                        _ => old - x,
                    };
                    let new = self.kept(new);
                    if let Some(slot) = self.slot(v) {
                        *slot = Val::Int(new);
                    }
                }
                S::Out(e) => {
                    let x = self.e(e)?;
                    let x = self.kept(x);
                    self.cards.push(x);
                }
                S::If(c, a, b) => {
                    let body = if self.c(c)? { a } else { b };
                    if self.block(body)? {
                        return Ok(true);
                    }
                }
                S::While(k, bound, body) => {
                    let n = self.env.len();
                    self.env.push((k.clone(), Val::Int(0)));
                    loop {
                        self.tick()?;
                        let (i, b) = (self.int(k), self.e(bound)?);
                        let b = self.kept(b);
                        if i >= b || self.block(body)? {
                            break;
                        }
                        let i = self.int(k) + 1;
                        *self.slot(k).unwrap() = Val::Int(i);
                    }
                    self.env.truncate(n);
                }
                S::For(i, lo, hi, _, body) => {
                    let (lo, hi) = (self.e(lo)?, self.e(hi)?);
                    let (lo, hi) = (self.kept(lo), self.kept(hi));
                    for v in lo..hi {
                        self.tick()?;
                        self.env.push((i.clone(), Val::Int(v)));
                        let broke = self.block(body);
                        self.env.pop();
                        if broke? {
                            break;
                        }
                    }
                }
                S::Break(c) => {
                    if self.c(c)? {
                        return Ok(true);
                    }
                }
                S::Array(a, vals) => self.env.push((a.clone(), Val::Array(vals.clone()))),
                S::Store(a, ix, e) => {
                    let i = self.ix(ix)?;
                    let x = self.e(e)?;
                    let x = self.kept(x);
                    match self.slot(a) {
                        Some(Val::Array(t)) => {
                            let i = usize::try_from(i).map_err(|_| End::Panic)?;
                            *t.get_mut(i).ok_or(End::Panic)? = x;
                        }
                        _ => return Err(End::Panic),
                    }
                }
                S::Bump(f, v, e) => {
                    let d = self.e(e)?;
                    let d = self.kept(d);
                    let x = self.int(v);
                    let func = self.p.funcs.iter().find(|g| &g.name == f).unwrap();
                    let saved = std::mem::take(&mut self.env);
                    self.env = vec![("*v".to_string(), Val::Int(x)), ("d".to_string(), Val::Int(d))];
                    let r = self.block(&func.body);
                    let after = self.int("*v");
                    let after = self.kept(after);
                    self.env = saved;
                    r?;
                    if let Some(slot) = self.slot(v) {
                        *slot = Val::Int(after);
                    }
                }
                S::Assume(c) => {
                    if !self.c(c)? {
                        return Err(End::Broken);
                    }
                }
            }
        }
        Ok(false)
    }
}

/// Run the program on this input as Rust would, following each value as a number of any size.
pub fn simulate(p: &Prog, input: &[i64]) -> Sim {
    let mut m = Machine {
        p,
        input,
        read: 0,
        cards: vec![],
        past: false,
        past_any: false,
        steps: 0,
        env: vec![],
    };
    let end = match m.block(&p.main) {
        Ok(_) => End::Done,
        Err(e) => e,
    };
    Sim {
        cards: m.cards,
        end,
        past: m.past,
        past_any: m.past_any,
    }
}

// Shrinking a program: each way of making it a little smaller.

/// Visit every statement list, depth first; `f` is given each list and may change it.
fn lists(p: &mut Prog, f: &mut dyn FnMut(&mut Vec<S>)) {
    fn walk(list: &mut Vec<S>, f: &mut dyn FnMut(&mut Vec<S>)) {
        f(list);
        for s in list.iter_mut() {
            match s {
                S::If(_, a, b) => {
                    walk(a, f);
                    walk(b, f);
                }
                S::While(_, _, b) | S::For(_, _, _, _, b) => walk(b, f),
                _ => {}
            }
        }
    }
    for func in &mut p.funcs {
        walk(&mut func.body, f);
    }
    walk(&mut p.main, f);
}

/// Statement `k` (counting depth first): what `f` makes of the list it's in and its place there.
fn at_stmt(p: &Prog, k: usize, f: &dyn Fn(&mut Vec<S>, usize) -> bool) -> Option<Prog> {
    let mut q = p.clone();
    let mut seen = 0;
    let (mut hit, mut done) = (false, false);
    lists(&mut q, &mut |list| {
        if hit {
            return;
        }
        if k < seen + list.len() {
            hit = true;
            done = f(list, k - seen);
        } else {
            seen += list.len();
        }
    });
    done.then_some(q)
}

fn count_stmts(p: &Prog) -> usize {
    let mut n = 0;
    lists(&mut p.clone(), &mut |list| n += list.len());
    n
}

/// Visit every expression and condition the shrinker may change, depth first. The inputs' reads and the promises
/// are left as they are: changed, a run would read other inputs, or a promise its input doesn't keep.
fn exprs(p: &mut Prog, f: &mut dyn FnMut(Node)) {
    fn e(x: &mut E, f: &mut dyn FnMut(Node)) {
        if matches!(x, E::Call(n, _) if n == "inp") {
            return;
        }
        f(Node::E(&mut *x));
        match x {
            E::Neg(a) | E::Abs(a) => e(a, f),
            E::Bin(_, a, b) => {
                e(a, f);
                e(b, f);
            }
            E::If(c0, a, b) => {
                c(c0, f);
                e(a, f);
                e(b, f);
            }
            E::Call(_, args) | E::Enum(_, _, args, _) => args.iter_mut().for_each(|a| e(a, f)),
            E::Index(_, ix) => {
                if let Ix::Cast(i, _) = &mut **ix {
                    e(i, f)
                }
            }
            _ => {}
        }
    }
    fn c(x: &mut C, f: &mut dyn FnMut(Node)) {
        f(Node::C(&mut *x));
        match x {
            C::Cmp(_, a, b) => {
                e(a, f);
                e(b, f);
            }
            C::And(a, b) | C::Or(a, b) => {
                c(a, f);
                c(b, f);
            }
            C::Not(a) => c(a, f),
            _ => {}
        }
    }
    fn s(x: &mut S, f: &mut dyn FnMut(Node)) {
        match x {
            S::Let(_, a) | S::Set(_, _, a) | S::Out(a) | S::Bump(_, _, a) => e(a, f),
            S::Flag(_, a) | S::Break(a) => c(a, f),
            S::If(a, t, o) => {
                c(a, f);
                t.iter_mut().for_each(|x| s(x, f));
                o.iter_mut().for_each(|x| s(x, f));
            }
            S::While(_, b, body) => {
                e(b, f);
                body.iter_mut().for_each(|x| s(x, f));
            }
            S::For(_, lo, hi, _, body) => {
                e(lo, f);
                e(hi, f);
                body.iter_mut().for_each(|x| s(x, f));
            }
            S::Store(_, ix, a) => {
                if let Ix::Cast(i, _) = ix {
                    e(i, f)
                }
                e(a, f)
            }
            S::Array(..) | S::Assume(_) => {}
        }
    }
    for func in &mut p.funcs {
        func.body.iter_mut().for_each(|x| s(x, f));
        if let Some(r) = &mut func.ret {
            e(r, f);
        }
    }
    p.main.iter_mut().for_each(|x| s(x, f));
}

enum Node<'a> {
    E(&'a mut E),
    C(&'a mut C),
}

/// The simpler things an expression or a condition can become: a constant, or a part of it.
fn simpler_e(x: &E) -> Vec<E> {
    let mut v = vec![];
    if !matches!(x, E::Num(0)) {
        v.push(E::Num(0));
    }
    if !matches!(x, E::Num(_)) {
        v.push(E::Num(1));
    }
    match x {
        E::Neg(a) | E::Abs(a) => v.push((**a).clone()),
        E::Bin(_, a, b) | E::If(_, a, b) => {
            v.push((**a).clone());
            v.push((**b).clone());
        }
        E::Call(_, args) | E::Enum(_, _, args, _) => v.extend(args.iter().cloned()),
        E::Num(n) if n.abs() > 1 => v.push(E::Num(n / 2)),
        _ => {}
    }
    v
}

fn simpler_c(x: &C) -> Vec<C> {
    let mut v = vec![];
    if !matches!(x, C::Lit(_)) {
        v.push(C::Lit(true));
        v.push(C::Lit(false));
    }
    match x {
        C::And(a, b) | C::Or(a, b) => {
            v.push((**a).clone());
            v.push((**b).clone());
        }
        C::Not(a) => v.push((**a).clone()),
        _ => {}
    }
    v
}

/// Every program one step smaller than p, the biggest steps first.
pub fn smaller(p: &Prog) -> Vec<Prog> {
    let mut out = vec![];
    for i in 0..p.funcs.len() {
        let mut q = p.clone();
        q.funcs.remove(i);
        out.push(q);
        if p.funcs[i].never {
            let mut q = p.clone();
            q.funcs[i].never = false;
            out.push(q);
        }
    }
    for i in 0..p.tables.len() {
        let mut q = p.clone();
        q.tables.remove(i);
        out.push(q);
    }
    for i in 0..p.consts.len() {
        let mut q = p.clone();
        q.consts.remove(i);
        out.push(q);
    }
    // the last input read: the run's input then has a number left over, which changes nothing
    let reads = |s: &S| matches!(s, S::Let(_, E::Call(f, _)) if f == "inp");
    if let Some(i) = p.main.iter().rposition(reads) {
        let mut q = p.clone();
        q.main.remove(i);
        q.inputs.pop();
        out.push(q);
    }
    let n = count_stmts(p);
    for k in 0..n {
        // the statement gone, or what's inside it in its place
        let gone = at_stmt(p, k, &|list, i| {
            if reads(&list[i]) {
                return false;
            }
            list.remove(i);
            true
        });
        out.extend(gone);
        for side in 0..2 {
            out.extend(at_stmt(p, k, &|list, i| {
                let inner = match &list[i] {
                    S::If(_, a, _) if side == 0 => a.clone(),
                    S::If(_, _, b) if side == 1 => b.clone(),
                    S::While(_, _, b) | S::For(_, _, _, _, b) if side == 0 => b.clone(),
                    _ => return false,
                };
                list.splice(i..=i, inner).for_each(drop);
                true
            }));
        }
        out.extend(at_stmt(p, k, &|list, i| match &mut list[i] {
            S::If(_, _, b) if !b.is_empty() => {
                b.clear();
                true
            }
            _ => false,
        }));
    }
    let mut count = 0;
    exprs(&mut p.clone(), &mut |_| count += 1);
    for k in 0..count {
        let mut subs = 0;
        let mut seen = 0;
        exprs(&mut p.clone(), &mut |node| {
            if seen == k {
                subs = match node {
                    Node::E(e) => simpler_e(e).len(),
                    Node::C(c) => simpler_c(c).len(),
                };
            }
            seen += 1;
        });
        for j in 0..subs {
            let mut q = p.clone();
            let mut seen = 0;
            exprs(&mut q, &mut |node| {
                if seen == k {
                    match node {
                        Node::E(e) => *e = simpler_e(e).swap_remove(j),
                        Node::C(c) => *c = simpler_c(c).swap_remove(j),
                    }
                }
                seen += 1;
            });
            out.push(q);
        }
    }
    out
}

// Building a batch.

/// How a program's build went in one setting.
#[derive(Clone, PartialEq)]
pub enum Built {
    Yes,
    /// rustc refused it: the generator's mistake.
    Rustc(String),
    /// More than CARDIAC's memory holds.
    TooBig,
    /// The compiler refused it otherwise, or panicked, or hung: a finding (or a known issue).
    Refused(String),
}

/// A package of these programs (name, source), its profiles the settings'.
fn write_programs(root: &Path, pkg: &Path, programs: &[(String, String)]) {
    std::fs::create_dir_all(pkg).unwrap();
    let names: Vec<String> = programs.iter().map(|p| p.0.clone()).collect();
    for (name, text) in programs {
        let to = pkg.join(format!("{name}.rs"));
        if std::fs::read_to_string(&to).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&to, text).unwrap();
        }
    }
    let toml = manifest(root, &names);
    let path = pkg.join("Cargo.toml");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(toml.as_str()) {
        std::fs::write(&path, toml).unwrap();
    }
}

/// The program's name in a line of the compiler's: its source's file name.
fn named_in<'a>(line: &str, names: &'a [String]) -> Option<&'a String> {
    names.iter().find(|n| line.contains(&format!("/{n}.rs:")))
}

/// What a refusal says, without where (the function, the MIR) and the numbers that differ from program to
/// program: to tell duplicates apart.
pub fn gist(why: &str) -> String {
    let why = why.split(".rs: ").last().unwrap_or(why);
    let why = why.split(", in ").next().unwrap();
    // and the MIR it quotes, as in "division into a computed place: _3[_9] = Div(...)"
    let why = why.split(": _").next().unwrap().split(": (").next().unwrap();
    let mut s = String::new();
    let mut last_digit = false;
    for ch in why.chars() {
        if ch.is_ascii_digit() {
            if !last_digit {
                s.push('N');
            }
            last_digit = true;
        } else {
            s.push(ch);
            last_digit = false;
        }
    }
    s.chars().take(100).collect()
}

/// Build these programs of the package in these settings, natively with one cargo build each and for CARDIAC with
/// as few `cargo cardiac build`s as the refusals allow. Each deck goes to work.dir/SETTING/.
pub fn build_all(
    work: &Work,
    names: &[String],
    settings: &[&'static str],
) -> HashMap<&'static str, HashMap<String, Built>> {
    let manifest = work.pkg.join("Cargo.toml");
    let mut all = HashMap::new();
    for &setting in settings {
        let mut result: HashMap<String, Built> = HashMap::new();
        for n in names {
            let _ = std::fs::remove_file(work.native(setting, n));
            let _ = std::fs::remove_file(work.pkg.join(format!("{n}.cqb")));
            let _ = std::fs::remove_file(work.deck(setting, n));
        }
        let out = Command::new("cargo")
            .args([
                "build",
                "-q",
                "--keep-going",
                "--message-format=json",
                "--profile",
                setting,
            ])
            .arg("--manifest-path")
            .arg(&manifest)
            .args(names.iter().flat_map(|n| ["--bin", n.as_str()]))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        let json = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() && !json.contains("\"level\":\"error\"") {
            die(format!(
                "the native build failed ({setting}):\n{}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let mut ok = vec![];
        for n in names {
            if work.native(setting, n).exists() {
                ok.push(n.clone());
                continue;
            }
            // the first error rustc gave for it
            let why = json
                .lines()
                .filter(|l| l.contains("\"level\":\"error\"") && l.contains(&format!("\"name\":\"{n}\"")))
                .find_map(|l| {
                    l.split("\"rendered\":\"")
                        .nth(1)
                        .map(|r| r.split("\\n").next().unwrap().to_string())
                })
                .unwrap_or_else(|| "no binary".into());
            result.insert(n.clone(), Built::Rustc(why));
        }
        std::fs::create_dir_all(work.dir.join(setting)).unwrap();
        let mut left = ok;
        while !left.is_empty() {
            let mut cmd = Command::new(&work.tool);
            cmd.args(["build", "-q", "--profile", setting, "--manifest-path"])
                .arg(&manifest);
            for n in &left {
                cmd.args(["--bin", n]);
            }
            let r = run_with(cmd, "", Duration::from_secs(60 + 2 * left.len() as u64));
            let mut rest = vec![];
            let mut blamed: Option<String> = None;
            let errors: Vec<&str> = r.why.lines().filter(|l| l.starts_with("error")).collect();
            if r.status != 0 {
                blamed = errors.iter().find_map(|l| named_in(l, &left)).cloned();
            }
            for n in &left {
                if work.pkg.join(format!("{n}.cqb")).exists() {
                    work.keep(setting, n);
                    result.insert(n.clone(), Built::Yes);
                } else {
                    rest.push(n.clone());
                }
            }
            if r.status == 0 || rest.is_empty() {
                for n in rest {
                    result.insert(n, Built::Refused("no deck".into()));
                }
                break;
            }
            // the one it stopped at: named in the error, or else the first without a deck
            let culprit = blamed.filter(|b| rest.contains(b)).unwrap_or_else(|| rest[0].clone());
            let why = match r.status {
                -1 => "the compiler hung".to_string(),
                -2 => format!(
                    "the compiler was killed by a signal: {}",
                    r.why.lines().last().unwrap_or("")
                ),
                _ if r.why.contains("panicked") || r.why.contains("overflowed its stack") => {
                    let l = r
                        .why
                        .lines()
                        .find(|l| l.contains("panicked") || l.contains("overflow"))
                        .unwrap();
                    format!("the compiler crashed: {l}")
                }
                _ => errors
                    .last()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| r.why.trim().to_string()),
            };
            let built = if why.contains("more than CARDIAC's memory holds") {
                Built::TooBig
            } else if left.len() > 1 && why.contains("cargo check failed") {
                // not one program's: build each alone
                for n in &rest {
                    let one = build_all_one(work, n, setting);
                    result.insert(n.clone(), one);
                }
                break;
            } else {
                Built::Refused(why)
            };
            result.insert(culprit.clone(), built);
            rest.retain(|n| *n != culprit);
            left = rest;
        }
        all.insert(setting, result);
    }
    all
}

/// One program alone for CARDIAC, when a batch can't say which failed.
fn build_all_one(work: &Work, name: &str, setting: &'static str) -> Built {
    let mut cmd = Command::new(&work.tool);
    cmd.args(["build", "-q", "--profile", setting, "--bin", name, "--manifest-path"])
        .arg(work.pkg.join("Cargo.toml"));
    let r = run_with(cmd, "", Duration::from_secs(120));
    if r.status == 0 {
        work.keep(setting, name);
        return Built::Yes;
    }
    let why = r
        .why
        .lines()
        .filter(|l| l.starts_with("error"))
        .last()
        .unwrap_or("failed")
        .to_string();
    if why.contains("more than CARDIAC's memory holds") {
        Built::TooBig
    } else {
        Built::Refused(why)
    }
}

/// The programs built in each setting, as stage 1's `differs` takes them.
fn built_map(b: &HashMap<&'static str, HashMap<String, Built>>) -> HashMap<&'static str, Vec<String>> {
    SETTINGS
        .iter()
        .map(|(s, _, _)| {
            let names = b
                .get(s)
                .map(|m| {
                    m.iter()
                        .filter(|(_, v)| **v == Built::Yes)
                        .map(|(n, _)| n.clone())
                        .collect()
                })
                .unwrap_or_default();
            (*s, names)
        })
        .collect()
}

// A campaign.

/// What a finding is, to group duplicates.
#[derive(Clone)]
pub struct Finding {
    pub what: String,
    pub program: String,
    pub report: String,
    pub files: Vec<PathBuf>,
}

#[derive(Default)]
struct Counts {
    programs: usize,
    /// those whose source has an enum kit's types or helpers
    enums: usize,
    rustc: usize,
    rustc_examples: Vec<String>,
    /// For each setting: built, too big, refused for a cast to usize, refused otherwise, refused for a constant past a card
    /// that every run computes.
    built: HashMap<&'static str, (usize, usize, usize, usize, usize)>,
    runs: usize,
    same: usize,
    past: usize,
    failed: usize,
    /// Programs every setting refused as too big.
    never: usize,
    /// Runs whose input broke a promise of the program (the generator's mistake), and runs where the generator's
    /// own run of the program and Rust's disagree (the oracle's), with a few of them.
    broken: usize,
    disagree: usize,
    disagree_examples: Vec<String>,
}

/// What a program's runs said of the generator's own run of it.
#[derive(Default)]
struct Oracle {
    broken: usize,
    disagree: usize,
    example: Option<String>,
}

fn ints(values: &[Value]) -> Vec<i64> {
    values.iter().map(|v| v.v).collect()
}

/// A run both ways.
fn both(work: &Work, setting: &str, name: &str, values: &[Value]) -> (Run, Run) {
    (
        work.run_native(setting, name, values),
        work.run_cardiac(setting, name, values),
    )
}

/// Whether Rust's run is the one the generator expects: the same cards, and a panic (its -999 card, exit 101)
/// where the generator's run panicked. A run that passed a card, or ran too long, is not compared.
fn agrees(sim: &Sim, native: &Run) -> bool {
    if sim.past_any || sim.end == End::TooLong {
        return true;
    }
    let mut cards: Vec<String> = sim.cards.iter().map(|c| c.to_string()).collect();
    if sim.end == End::Panic {
        cards.push("-999".into());
    }
    let status = if sim.end == End::Panic { 101 } else { 0 };
    native.status == status && native.cards.split_whitespace().eq(cards.iter().map(|c| c.as_str()))
}

/// Whether a run of a program is a miscompile: CARDIAC differs from Rust, Rust is what the generator expects, and
/// no value passed a card on the way.
fn miscompiled(p: &Prog, work: &Work, setting: &str, name: &str, values: &[Value]) -> Option<(Run, Run)> {
    let sim = simulate(p, &ints(values));
    let checks = SETTINGS.iter().any(|(s, _, c)| *s == setting && *c);
    if sim.past_in(checks) || sim.end == End::Broken {
        return None;
    }
    let (native, cardiac) = both(work, setting, name, values);
    let same = (native.cards.as_str(), native.status) == (cardiac.cards.as_str(), cardiac.status);
    (!same && agrees(&sim, &native)).then_some((native, cardiac))
}

/// The smallest input, as far as a greedy search finds, on which the program is still miscompiled: the last
/// value gone (the others keep their places, so their promises), or a value nearer 0 in its range.
fn shrink_input(p: &Prog, work: &Work, setting: &str, name: &str, mut values: Vec<Value>) -> Vec<Value> {
    let mut tries = 0;
    'better: loop {
        let mut candidates: Vec<Vec<Value>> = vec![];
        if !values.is_empty() {
            candidates.push(values[..values.len() - 1].to_vec());
        }
        for i in 0..values.len() {
            let x = values[i];
            for v in [0, x.v / 2, x.v - x.v.signum()] {
                let v = v.clamp(x.lo, x.hi);
                if v.abs() < x.v.abs() {
                    let mut nearer = values.clone();
                    nearer[i].v = v;
                    candidates.push(nearer);
                }
            }
        }
        for c in candidates {
            tries += 1;
            if tries > 300 {
                break 'better;
            }
            if miscompiled(p, work, setting, name, &c).is_some() {
                values = c;
                continue 'better;
            }
        }
        break;
    }
    values
}

pub struct Options {
    pub count: u64,
    pub start: u64,
    pub seed: u64,
    pub runs: u64,
    pub batch: usize,
    pub shrink: bool,
    /// The percentage of programs with enum kits.
    pub enums: u64,
}

/// What the compiler says of a constant no card holds.
const CONSTANT: &str = "constant out of range";

/// Known issues, by what the compiler says.
fn known(why: &str) -> Option<&'static str> {
    // enums' parts still to come: Option's and Result's methods where rustc calls them (opt-level 1 and s),
    // and what the compiler says it doesn't support yet
    let enums_to_come = [
        "unsupported call to Option::",
        "unsupported call to Result::",
        "as Try>::branch",
        "from_residual",
    ];
    if why.contains("unsupported place") && why.contains("as usize") {
        Some("usize place")
    } else if why.contains("enum with data")
        || why.contains("the fields of an enum's variants")
        || enums_to_come.iter().any(|w| why.contains(w))
        || (why.contains("behind a reference") && why.contains("__enum"))
    {
        Some("enum")
    } else {
        None
    }
}

/// Generate, build and run programs start..start+count of the seed, a batch at a time; report and exit.
pub fn campaign(root: &Path, tool: &Path, o: Options) {
    let dir = root.join("target/fuzz/gen");
    let work = Work {
        tool: tool.to_path_buf(),
        pkg: dir.join("pkg"),
        dir: dir.clone(),
    };
    let findings_dir = root.join("target/fuzz/findings");
    std::fs::create_dir_all(&findings_dir).unwrap();
    let mut counts = Counts::default();
    let mut findings: Vec<Finding> = vec![];
    let started = Instant::now();
    let settings: Vec<&'static str> = SETTINGS.iter().map(|s| s.0).collect();
    println!(
        "-- generated programs {}..{} of seed {}, {} runs each in {} settings, {} a batch",
        o.start,
        o.start + o.count,
        o.seed,
        o.runs,
        SETTINGS.len(),
        o.batch
    );
    let mut n = o.start;
    while n < o.start + o.count {
        let end = (n + o.batch as u64).min(o.start + o.count);
        let progs: Vec<(String, u64, Prog)> = (n..end)
            .enumerate()
            .map(|(slot, k)| (format!("p{slot:03}"), k, program(o.seed, k, o.enums)))
            .collect();
        let sources: Vec<(String, String)> = progs
            .iter()
            .map(|(name, k, p)| {
                let head = format!(
                    "Generated by tests/fuzz/fuzz.rs: seed {} program {k}; inputs {}",
                    o.seed,
                    p.input_words()
                );
                (name.clone(), p.source(&head))
            })
            .collect();
        write_programs(root, &work.pkg, &sources);
        let names: Vec<String> = progs.iter().map(|p| p.0.clone()).collect();
        let b = build_all(&work, &names, &settings);
        let built = built_map(&b);
        counts.programs += progs.len();
        counts.enums += sources.iter().filter(|(_, s)| s.contains("fn ek")).count();
        for (name, k, p) in &progs {
            let first = &b[settings[0]][name];
            if let Built::Rustc(why) = first {
                counts.rustc += 1;
                if counts.rustc_examples.len() < 5 {
                    counts
                        .rustc_examples
                        .push(format!("seed {} program {k}: {why}", o.seed));
                }
                continue;
            }
            if settings.iter().all(|s| b[s][name] == Built::TooBig) {
                counts.never += 1;
            }
            let mut refused: Vec<(String, &'static str)> = vec![];
            for &s in &settings {
                let c = counts.built.entry(s).or_default();
                match &b[s][name] {
                    Built::Yes => c.0 += 1,
                    Built::TooBig => c.1 += 1,
                    Built::Refused(why) if known(why).is_some() => c.2 += 1,
                    // a constant past a card: an overflow known at compile time, an error as rustc's own
                    // `arithmetic_overflow` is, whether or not a run computes it: not a finding
                    Built::Refused(why) if why.contains(CONSTANT) => c.4 += 1,
                    Built::Refused(why) => {
                        c.3 += 1;
                        refused.push((why.clone(), s));
                    }
                    Built::Rustc(_) => {}
                }
            }
            // one finding a program for refusals, its settings listed
            if let Some((why, _)) = refused.first() {
                let settings: Vec<String> = refused.iter().map(|r| r.1.to_string()).collect();
                // the first two of a kind are shrunk; the rest only counted, as duplicates
                let again = findings
                    .iter()
                    .filter(|f| f.what == format!("refused: {}", gist(why)))
                    .count();
                let shrink = o.shrink && again < 2;
                let f = refusal(
                    root,
                    &work,
                    &findings_dir,
                    (o.seed, *k, o.runs),
                    p,
                    &sources,
                    name,
                    why,
                    refused[0].1,
                    &settings,
                    shrink,
                );
                println!("{}", f.report);
                findings.push(f);
            }
        }
        // the runs: a thread a core, a program at a time
        let next = AtomicUsize::new(0);
        type Outcome = (
            usize,
            Vec<(usize, usize, usize)>,
            Option<(&'static str, Vec<Value>, u64)>,
            Oracle,
        );
        let results: Mutex<Vec<Outcome>> = Mutex::new(vec![]);
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        std::thread::scope(|sc| {
            for _ in 0..threads {
                sc.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= progs.len() {
                        break;
                    }
                    let (name, k, p) = &progs[i];
                    let t = p.target(&format!("generated-{k}"));
                    let mut tallies = vec![(0, 0, 0); SETTINGS.len()];
                    let mut first: Option<(&'static str, Vec<Value>, u64)> = None;
                    let mut oracle = Oracle::default();
                    for r in 0..o.runs {
                        let values = super::generate(&t, o.seed, r);
                        let sim = simulate(p, &ints(&values));
                        if sim.end == End::Broken {
                            oracle.broken += 1;
                            continue;
                        }
                        let (mut checked, mut trusted) = (false, true);
                        for (j, (setting, _, checks)) in SETTINGS.iter().enumerate() {
                            if !built[setting].contains(name) {
                                continue;
                            }
                            let (native, cardiac) = both(&work, setting, name, &values);
                            // the generator's own run against Rust's, once a run: a check on the oracle
                            // (a run on which they disagree is left out: the oracle can't judge it)
                            if !checked {
                                checked = true;
                                if !agrees(&sim, &native) {
                                    trusted = false;
                                    oracle.disagree += 1;
                                    if oracle.example.is_none() {
                                        oracle.example = Some(format!(
                                            "seed {} program {k} input {}: rust {} / generator {:?} {:?}",
                                            o.seed,
                                            words(&values).join(" "),
                                            brief(&native),
                                            sim.cards,
                                            sim.end
                                        ));
                                    }
                                }
                            }
                            if !trusted {
                                break;
                            }
                            if (native.cards.as_str(), native.status) == (cardiac.cards.as_str(), cardiac.status) {
                                tallies[j].0 += 1;
                            } else if sim.past_in(*checks) {
                                tallies[j].1 += 1;
                            } else {
                                tallies[j].2 += 1;
                                if first.is_none() {
                                    first = Some((setting, values.clone(), r));
                                }
                            }
                        }
                    }
                    results.lock().unwrap().push((i, tallies, first, oracle));
                });
            }
        });
        let mut results = results.into_inner().unwrap();
        results.sort_by_key(|r| r.0);
        for (i, tallies, first, oracle) in results {
            counts.broken += oracle.broken;
            counts.disagree += oracle.disagree;
            if let Some(e) = oracle.example {
                if counts.disagree_examples.len() < 5 {
                    counts.disagree_examples.push(e);
                }
            }
            for t in &tallies {
                counts.runs += t.0 + t.1 + t.2;
                counts.same += t.0;
                counts.past += t.1;
                counts.failed += t.2;
            }
            if let Some((setting, values, r)) = first {
                let (name, k, p) = &progs[i];
                let failing: Vec<String> = SETTINGS
                    .iter()
                    .zip(&tallies)
                    .filter(|(_, t)| t.2 > 0)
                    .map(|((s, _, _), t)| format!("{s} ({} runs)", t.2))
                    .collect();
                // as with refusals, the first two of a kind are shrunk
                let (native, cardiac) = both(&work, setting, name, &values);
                let what = format!("differs: native {} / cardiac {}", short(&native), short(&cardiac));
                let shrink = o.shrink && findings.iter().filter(|f| f.what == what).count() < 2;
                let f = difference(
                    root,
                    &work,
                    &findings_dir,
                    o.seed,
                    *k,
                    r,
                    p,
                    &sources,
                    name,
                    setting,
                    values,
                    failing,
                    shrink,
                );
                println!("{}", f.report);
                findings.push(f);
            }
        }
        n = end;
        println!(
            "-- {} programs so far ({} s): {} runs compared, {} the same, {} past a card, {} differ; {} findings",
            counts.programs,
            started.elapsed().as_secs(),
            counts.runs,
            counts.same,
            counts.past,
            counts.failed,
            findings.len()
        );
    }
    println!(
        "\n-- {} programs generated, seed {}, programs {}..{}",
        counts.programs,
        o.seed,
        o.start,
        o.start + o.count
    );
    println!("   with enums: {} (--enums {})", counts.enums, o.enums);
    let valid = counts.programs - counts.rustc;
    println!(
        "   rustc refused {} ({:.1}%){}",
        counts.rustc,
        100.0 * counts.rustc as f64 / counts.programs.max(1) as f64,
        if counts.rustc_examples.is_empty() {
            String::new()
        } else {
            format!(": {}", counts.rustc_examples.join("; "))
        }
    );
    println!(
        "   too big in every setting: {} ({:.1}% of those rustc built)",
        counts.never,
        100.0 * counts.never as f64 / valid.max(1) as f64
    );
    println!(
        "   {:<16}{:>8}{:>10}{:>8}{:>9}{:>16}",
        "setting", "built", "too big", "usize", "refused", "past (constant)"
    );
    for s in &settings {
        let c = counts.built.get(s).copied().unwrap_or_default();
        println!("   {s:<16}{:>8}{:>10}{:>8}{:>9}{:>16}", c.0, c.1, c.2, c.3, c.4);
    }
    println!(
        "   runs compared: {}; the same {}, past a card {} ({:.1}%), differ {}",
        counts.runs,
        counts.same,
        counts.past,
        100.0 * counts.past as f64 / counts.runs.max(1) as f64,
        counts.failed
    );
    println!(
        "   runs left out: their input breaking a promise {}; the generator's run and Rust's disagreeing {}{}",
        counts.broken,
        counts.disagree,
        if counts.disagree_examples.is_empty() {
            String::new()
        } else {
            format!(": {}", counts.disagree_examples.join("; "))
        }
    );
    let mut groups: Vec<(String, Vec<&Finding>)> = vec![];
    for f in &findings {
        match groups.iter_mut().find(|g| g.0 == f.what) {
            Some(g) => g.1.push(f),
            None => groups.push((f.what.clone(), vec![f])),
        }
    }
    println!("   findings: {} in {} kinds", findings.len(), groups.len());
    for (what, fs) in &groups {
        let progs: Vec<&str> = fs.iter().map(|f| f.program.as_str()).collect();
        println!("     {what}: {}", progs.join(", "));
        println!("       {}", fs[0].files[1].display());
    }
    if !findings.is_empty() {
        std::process::exit(1);
    }
}

/// Save a finding's files: the program as generated, as shrunk, and what was found.
fn save(dir: &Path, stem: &str, original: &str, small: &str, report: &str) -> Vec<PathBuf> {
    let files = vec![
        dir.join(format!("{stem}.rs")),
        dir.join(format!("{stem}-small.rs")),
        dir.join(format!("{stem}.txt")),
    ];
    std::fs::write(&files[0], original).unwrap();
    std::fs::write(&files[1], small).unwrap();
    std::fs::write(&files[2], report).unwrap();
    files
}

/// A refusal: shrunk while the compiler still refuses it the same way in that setting.
#[allow(clippy::too_many_arguments)]
fn refusal(
    root: &Path,
    work: &Work,
    dir: &Path,
    (seed, k, _): (u64, u64, u64),
    p: &Prog,
    sources: &[(String, String)],
    name: &str,
    why: &str,
    setting: &'static str,
    settings: &[String],
    shrink: bool,
) -> Finding {
    let what = format!("refused: {}", gist(why));
    let head = format!(
        "Generated by tests/fuzz/fuzz.rs: seed {seed} program {k}, shrunk; inputs {}",
        p.input_words()
    );
    let small = if shrink {
        let g = gist(why);
        shrink_program(
            root,
            work.tool.as_path(),
            p,
            &[setting],
            &|b, _, n, _| matches!(&b[setting][n], Built::Refused(w) if gist(w) == g),
        )
    } else {
        p.clone()
    };
    let original = &sources.iter().find(|s| s.0 == name).unwrap().1;
    let stem = format!("seed{seed}-{k}");
    let mut report = format!("REFUSED seed {seed} program {k} in {}:\n  {why}\n", settings.join(" "));
    let files_at = [dir.join(format!("{stem}.rs")), dir.join(format!("{stem}-small.rs"))];
    report += &format!(
        "  program: {} (shrunk: {})\n",
        files_at[0].display(),
        files_at[1].display()
    );
    report += &format!("  again:   sh fuzz.sh --repro {}\n", files_at[1].display());
    let files = save(dir, &stem, original, &small.source(&head), &report);
    Finding {
        what,
        program: format!("seed {seed} program {k}"),
        report,
        files,
    }
}

/// A run that differs: the input shrunk, then the program, while CARDIAC still differs from Rust on it with no
/// value past a card.
#[allow(clippy::too_many_arguments)]
fn difference(
    root: &Path,
    work: &Work,
    dir: &Path,
    seed: u64,
    k: u64,
    r: u64,
    p: &Prog,
    sources: &[(String, String)],
    name: &str,
    setting: &'static str,
    values: Vec<Value>,
    failing: Vec<String>,
    shrink: bool,
) -> Finding {
    let values = if shrink {
        shrink_input(p, work, setting, name, values)
    } else {
        values
    };
    let (native, cardiac) = both(work, setting, name, &values);
    let input = words(&values).join(" ");
    let small = if shrink {
        let vs = values.clone();
        shrink_program(root, work.tool.as_path(), p, &[setting], &|_, w, n, q| {
            w.deck(setting, n).exists() && miscompiled(q, w, setting, n, &vs).is_some()
        })
    } else {
        p.clone()
    };
    let original = &sources.iter().find(|s| s.0 == name).unwrap().1;
    let head = format!(
        "Generated by tests/fuzz/fuzz.rs: seed {seed} program {k}, shrunk; inputs {}",
        p.input_words()
    );
    let stem = format!("seed{seed}-{k}");
    let what = format!("differs: native {} / cardiac {}", short(&native), short(&cardiac));
    let mut report = format!(
        "DIFFER seed {seed} program {k} run {r} in {}: input {input}\n",
        failing.join(", ")
    );
    report += &format!("  native:  {}\n  cardiac: {}", brief(&native), brief(&cardiac));
    if cardiac.status == 101 {
        report += &format!(" ({})", cardiac.why);
    }
    let files_at = [dir.join(format!("{stem}.rs")), dir.join(format!("{stem}-small.rs"))];
    report += &format!(
        "\n  program: {} (shrunk: {})\n",
        files_at[0].display(),
        files_at[1].display()
    );
    if shrink {
        report += &match run_small(root, work.tool.as_path(), &small, setting, &values) {
            Some((n, c)) => format!("  shrunk:  native {} / cardiac {}\n", brief(&n), brief(&c)),
            None => "  shrunk:  (no longer differs alone)\n".into(),
        };
    }
    report += &format!("  again:   sh fuzz.sh --repro {} -- {input}\n", files_at[1].display());
    let files = save(dir, &stem, original, &small.source(&head), &report);
    Finding {
        what,
        program: format!("seed {seed} program {k}"),
        report,
        files,
    }
}

/// The shrunk program built alone and run on the input: what it does now.
fn run_small(root: &Path, tool: &Path, q: &Prog, setting: &'static str, values: &[Value]) -> Option<(Run, Run)> {
    let dir = root.join("target/fuzz/shrink");
    let work = Work {
        tool: tool.to_path_buf(),
        pkg: dir.join("pkg"),
        dir,
    };
    let name = "s000".to_string();
    write_programs(root, &work.pkg, &[(name.clone(), q.source("shrunk"))]);
    let b = build_all(&work, std::slice::from_ref(&name), &[setting]);
    (b[setting][&name] == Built::Yes)
        .then(|| miscompiled(q, &work, setting, &name, values))
        .flatten()
}

/// How a run ended, its cards cut short: a run that loops prints them for TIMEOUT.
fn brief(r: &Run) -> String {
    let cards: Vec<&str> = r.cards.split_whitespace().collect();
    if cards.len() <= 30 {
        return r.show();
    }
    let short = Run {
        cards: cards[..30].join(" "),
        status: r.status,
        why: String::new(),
    };
    format!("{} ({} cards in all)", short.show(), cards.len())
}

/// How a run ended, in a word or two, for grouping.
fn short(r: &Run) -> String {
    match r.status {
        0 => "exit 0".into(),
        101 => "panic".into(),
        -1 => "timed out".into(),
        s => format!("exit {s}"),
    }
}

/// Shrink a program while `still` holds of it: a round of smaller candidates at a time, built together in
/// target/fuzz/shrink/pkg in these settings; the smallest that still holds is kept, until none does.
fn shrink_program(
    root: &Path,
    tool: &Path,
    p: &Prog,
    set: &[&'static str],
    still: &(dyn Fn(&HashMap<&'static str, HashMap<String, Built>>, &Work, &str, &Prog) -> bool + Sync),
) -> Prog {
    let dir = root.join("target/fuzz/shrink");
    let work = Work {
        tool: tool.to_path_buf(),
        pkg: dir.join("pkg"),
        dir: dir.clone(),
    };
    let mut best = p.clone();
    let started = Instant::now();
    let mut tried: HashSet<String> = HashSet::new();
    'round: while started.elapsed() < Duration::from_secs(900) {
        let candidates: Vec<Prog> = smaller(&best)
            .into_iter()
            .filter(|q| tried.insert(q.source("")))
            .collect();
        for chunk in candidates.chunks(48) {
            let sources: Vec<(String, String)> = chunk
                .iter()
                .enumerate()
                .map(|(i, q)| (format!("s{i:03}"), q.source("shrinking")))
                .collect();
            write_programs(root, &work.pkg, &sources);
            let names: Vec<String> = sources.iter().map(|s| s.0.clone()).collect();
            let b = build_all(&work, &names, set);
            // each candidate tried on a thread of its own: one that loops takes TIMEOUT
            let (b, work, names) = (&b, &work, &names);
            let mut good: Vec<usize> = std::thread::scope(|sc| {
                let hs: Vec<_> = (0..chunk.len())
                    .map(|i| sc.spawn(move || still(b, work, &names[i], &chunk[i])))
                    .collect();
                hs.into_iter()
                    .enumerate()
                    .filter_map(|(i, h)| h.join().unwrap().then_some(i))
                    .collect()
            });
            good.sort_by_key(|&i| sources[i].1.len());
            if let Some(&i) = good.first() {
                best = chunk[i].clone();
                continue 'round;
            }
        }
        break;
    }
    best
}

/// Build one source in every setting and run it on this input both ways: how a finding is checked again, with
/// this compiler or another (--tool).
pub fn repro(root: &Path, tool: &Path, file: &Path, input: &[String]) {
    let text = std::fs::read_to_string(file).unwrap_or_else(|e| die(format!("{}: {e}", file.display())));
    let dir = root.join("target/fuzz/repro");
    let work = Work {
        tool: tool.to_path_buf(),
        pkg: dir.join("pkg"),
        dir,
    };
    let name = "repro".to_string();
    write_programs(root, &work.pkg, &[(name.clone(), text)]);
    let settings: Vec<&'static str> = SETTINGS.iter().map(|s| s.0).collect();
    let b = build_all(&work, std::slice::from_ref(&name), &settings);
    let built = built_map(&b);
    let values: Vec<Value> = input
        .iter()
        .map(|w| {
            let v = w.parse().unwrap_or_else(|_| die(format!("not a number: {w}")));
            Value { v, lo: v, hi: v }
        })
        .collect();
    println!(
        "-- {} with {}, input {}",
        file.display(),
        tool.display(),
        input.join(" ")
    );
    let one = Target { name: name.clone(), source: file.to_path_buf(), inputs: vec![], exact: false };
    let mut bad = false;
    for s in &settings {
        let line = match &b[s][&name] {
            Built::Yes => match differs(&work, &built, s, &one, &values) {
                None => format!("same: {}", brief(&work.run_native(s, &name, &values))),
                Some((n, c, past)) => {
                    bad |= !past;
                    let why = if c.status == 101 {
                        format!(" ({})", c.why)
                    } else {
                        String::new()
                    };
                    format!(
                        "{}: native {} / cardiac {}{why}",
                        if past { "past a card" } else { "DIFFER" },
                        brief(&n),
                        brief(&c)
                    )
                }
            },
            Built::TooBig => "too big".into(),
            Built::Rustc(w) => {
                bad = true;
                format!("rustc: {w}")
            }
            Built::Refused(w) => {
                bad |= known(w).is_none();
                format!("REFUSED: {w}")
            }
        };
        println!("{s:<16}{line}");
    }
    if bad {
        std::process::exit(1);
    }
}
