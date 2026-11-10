//! Range pass. Following the code in the order it runs, track a range for each local: from constants, copies,
//! arithmetic, and the branch a test took. An overflow check or a bounds check the ranges already prove is
//! dropped: `i < 3` on the way in proves both `xs[i]` in bounds for a [_; 4] and `i + 1` without overflow. Locals
//! whose address is taken are never tracked.
//!
//! Ranges are exact integers. Arithmetic on bounds saturates at i64's ends rather than overflow: a product of
//! products can outgrow any width, and a bound that far out still bounds every value a card can hold.
use std::collections::{HashMap, HashSet};

use super::ir::{
    predecessors, BasicBlock, BinOp, Block, CastKind, IntTy, Local, Locals, Operand, Place, Projection, Rvalue,
    Segment, Statement, Terminator, Ty, UnOp, Unwind,
};
use super::Fail;

pub type Range = [i64; 2];

/// What a range is known for: a local, or the value half `.0` of an overflow-checked pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    Local(Local),
    Value(Local),
}

/// What holds at a point: a range for each key. Edges: what holds on the way to each successor. Both are compared
/// as maps, not lists, so that the analysis settles once the facts stop changing, whatever order they came in; no
/// order of theirs reaches the output.
type Facts = HashMap<Key, Range>;
type Edges = HashMap<BasicBlock, Facts>;

/// The range of each type.
pub fn type_range(ty: &Ty) -> Option<Range> {
    match ty {
        Ty::Int(IntTy::U8) => Some([0, 255]),
        Ty::Int(IntTy::I8) => Some([-128, 127]),
        Ty::Int(IntTy::Usize) => Some([0, 999]),
        Ty::Int(IntTy::I16) => Some([-999, 999]),
        Ty::Bool => Some([0, 1]),
        _ => None,
    }
}

/// The range of each type natively, where Rust panics past it: wider than a card for i16 and usize. A checked op
/// whose value is never read can panic only there.
fn native_range(ty: &Ty) -> Option<Range> {
    match ty {
        Ty::Int(IntTy::U8) => Some([0, 255]),
        Ty::Int(IntTy::I8) => Some([-128, 127]),
        Ty::Int(IntTy::Usize) => Some([0, i64::MAX]),
        Ty::Int(IntTy::I16) => Some([i16::MIN as i64, i16::MAX as i64]),
        _ => None,
    }
}

/// The rounds in which a growing range is widened to the program's next constant; after them, to its type's end.
const STEPPING: usize = 20;

/// Unbounded, for a comparison's open side: far beyond any card.
const BIG: i64 = 1_000_000_000;

/// The range of a + b.
fn add(ra: Range, rb: Range) -> Range {
    [ra[0].saturating_add(rb[0]), ra[1].saturating_add(rb[1])]
}

/// The range of a - b.
fn sub(ra: Range, rb: Range) -> Range {
    [ra[0].saturating_sub(rb[1]), ra[1].saturating_sub(rb[0])]
}

/// The range of a op b from the ranges of a and b (Div and Rem only by a positive constant; else unknown).
fn arange(op: BinOp, ra: Range, rb: Range) -> Option<Range> {
    match op {
        BinOp::Add => return Some(add(ra, rb)),
        BinOp::Sub => return Some(sub(ra, rb)),
        BinOp::Mul => {
            let v = [
                ra[0].saturating_mul(rb[0]),
                ra[0].saturating_mul(rb[1]),
                ra[1].saturating_mul(rb[0]),
                ra[1].saturating_mul(rb[1]),
            ];
            return Some([*v.iter().min().unwrap(), *v.iter().max().unwrap()]);
        }
        _ => {}
    }
    if rb[0] != rb[1] || rb[0] <= 0 {
        return None;
    }
    let k = rb[0];
    if op == BinOp::Div {
        // truncates toward zero, as Rust's division does
        return Some([ra[0] / k, ra[1] / k]);
    }
    Some([if ra[0] < 0 { -(k - 1) } else { 0 }, if ra[1] > 0 { k - 1 } else { 0 }])
}

/// What a cell keeps of a value in `r`: past a card, its last three digits with its sign (1200 is kept as 200).
fn stored(r: Range) -> Range {
    let lo = if r[0] < -999 { -999 } else if r[1] > 999 { r[0].min(0) } else { r[0] };
    let hi = if r[1] > 999 { 999 } else if r[0] < -999 { r[1].max(0) } else { r[1] };
    [lo, hi]
}

/// What a local holds after a plain op whose exact result is in `r`, as the next statement reads it: the result
/// itself, carried there in the accumulator (see `carries` in lowering), or what its cell keeps of it.
fn kept(r: Range) -> Range {
    let s = stored(r);
    [r[0].min(s[0]), r[1].max(s[1])]
}

/// What a comparison `x op v` says about x when it holds, and when it does not.
fn held(op: BinOp, v: i64) -> Option<Range> {
    Some(match op {
        BinOp::Lt => [-BIG, v.saturating_sub(1)],
        BinOp::Le => [-BIG, v],
        BinOp::Gt => [v.saturating_add(1), BIG],
        BinOp::Ge => [v, BIG],
        _ => return None,
    })
}
fn not_held(op: BinOp, v: i64) -> Option<Range> {
    Some(match op {
        BinOp::Lt => [v, BIG],
        BinOp::Le => [v.saturating_add(1), BIG],
        BinOp::Gt => [-BIG, v],
        BinOp::Ge => [-BIG, v.saturating_sub(1)],
        _ => return None,
    })
}

fn is_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
    )
}

/// An operand as a comparison remembers it: the place without how it is read, or the constant.
fn bare(o: &Operand) -> Operand {
    match o {
        Operand::Copy(p) | Operand::Move(p) => Operand::Place(p.clone()),
        o => o.clone(),
    }
}

/// An integer constant's value, of any type.
fn int_value(o: &Operand) -> Option<i64> {
    o.int().map(|(v, _)| clamp(v))
}

/// A constant at i64's nearer end when it lies past it.
fn clamp(v: i128) -> i64 {
    v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// A plain unwind: `unwind continue` or `unwind unreachable`.
fn plain_unwind(u: &Unwind) -> bool {
    matches!(u, Unwind::Continue | Unwind::Unreachable)
}

/// `assert(!copy|move (_t.1: bool), ..) -> [success: bb]`: the check of an overflow-checked op's flag.
pub fn overflow_assert(t: &Terminator) -> Option<(Local, BasicBlock)> {
    let Terminator::Assert {
        cond: Operand::Copy(p) | Operand::Move(p),
        expected: false,
        target,
        unwind,
        ..
    } = t
    else {
        return None;
    };
    if !plain_unwind(unwind) {
        return None;
    }
    match p.projection.as_slice() {
        [Projection::Field(1, Ty::Bool)] => p.base_local().map(|l| (l, *target)),
        _ => None,
    }
}

/// `assert(copy|move _b, ..)` or `assert(!copy|move _b, ..)` on a whole local, with a plain unwind: the local,
/// whether the assert wants it true, and where it goes on.
fn assert_on_local(t: &Terminator) -> Option<(Local, bool, BasicBlock)> {
    let Terminator::Assert {
        cond: Operand::Copy(p) | Operand::Move(p),
        expected,
        target,
        unwind,
        ..
    } = t
    else {
        return None;
    };
    if !plain_unwind(unwind) {
        return None;
    }
    p.as_local().map(|l| (l, *expected, *target))
}

/// A locals's whole value assigned: `_x = ...`.
fn assigned_local(s: &Statement) -> Option<(Local, &Rvalue)> {
    let Statement::Assign(p, r) = s else { return None };
    p.as_local().filter(|l| *l != Local::UNIT).map(|l| (l, r))
}

/// The local whose value a statement changes, as far as the ranges follow: a whole local, or a field of one (the
/// first ten fields, of a type named in one word).
fn killed(s: &Statement) -> Option<Local> {
    let Statement::Assign(p, _) = s else { return None };
    let l = p.base_local().filter(|l| *l != Local::UNIT)?;
    match p.projection.as_slice() {
        [] => Some(l),
        [Projection::Field(k, t)] if *k < 10 && one_word(t) => Some(l),
        _ => None,
    }
}

/// A type printed as one word: `i16`, `bool`, `Point`.
fn one_word(t: &Ty) -> bool {
    match t {
        Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str => true,
        Ty::Path(p) => {
            p.qself.is_none() && p.segments.len() == 1 && matches!(&p.segments[0], Segment::Name { args: None, .. })
        }
        _ => false,
    }
}

/// The element type of an overflow-checked pair `(T, bool)`, when T is one word.
fn pair_value(ty: Option<&Ty>) -> Option<Ty> {
    match ty {
        Some(Ty::Tuple(ts)) if ts.len() == 2 && ts[1] == Ty::Bool && one_word(&ts[0]) => Some(ts[0].clone()),
        _ => None,
    }
}

struct Pass<'a> {
    locals: &'a HashMap<Local, Ty>,
    taken: HashSet<Local>,
    /// The overflow-checked pairs nothing reads but their own checks (see `unread_pairs`).
    unread: HashSet<Local>,
    /// The usize locals that can hold a value past a card, as a negative one (see `huge_usizes`).
    huge: HashSet<Local>,
    /// The usize locals a difference that can go below 0 is stored into (see `range_pass`).
    below: HashSet<Local>,
    dropped: usize,
    /// The arrays set once, from constants, and written nowhere else: the range of their elements (see
    /// `constant_tables`).
    tables: HashMap<Local, Range>,
    /// In the last visit: each plain sum, difference and product, and each linear combination, by block and
    /// statement, and whether its result may be past a card (see `past_a_card`).
    judged: Option<Vec<(BasicBlock, usize, Statement, bool)>>,
}

/// What a comparison into a bool remembers: the operator and both sides.
type Comparison = (BinOp, Operand, Operand);

impl Pass<'_> {
    fn of_type(&self, k: Key) -> Option<Range> {
        match k {
            Key::Local(x) if self.huge.contains(&x) => Some([-999, 999]),
            Key::Local(x) => self.locals.get(&x).and_then(type_range),
            Key::Value(_) => None,
        }
    }

    /// A checked op into x, statement i and the last of b, whose value r is past a card but which nothing reads: it
    /// is there only for its panic, and when r is within the type's native range it cannot panic, so op and check go.
    fn drop_unread(&mut self, b: &mut Block, i: usize, x: Local, r: Range, w: &Ty) {
        let checked = overflow_assert(&b.terminator).filter(|t| t.0 == x);
        let native = native_range(w).is_some_and(|[lo, hi]| r[0] >= lo && r[1] <= hi);
        if let (Some((_, target)), true, true) = (checked, native, self.unread.contains(&x)) {
            b.statements[i] = Statement::Nop;
            b.terminator = Terminator::Goto(target);
            self.dropped += 1;
        }
    }

    /// The range of an operand, if known: a constant of a card's type, or a whole local.
    fn val(&self, facts: &Facts, o: &Operand) -> Option<Range> {
        if let Some((v, t)) = o.int() {
            return matches!(t, IntTy::U8 | IntTy::I8 | IntTy::Usize | IntTy::I16).then_some([clamp(v), clamp(v)]);
        }
        if let Some(r) = self.element(o) {
            return Some(r);
        }
        let x = o.local()?;
        facts
            .get(&Key::Local(x))
            .copied()
            .or_else(|| self.of_type(Key::Local(x)))
    }

    /// An element of a constant table, `copy _1[_5]`: the range of its values.
    fn element(&self, o: &Operand) -> Option<Range> {
        let (Operand::Copy(p) | Operand::Move(p)) = o else {
            return None;
        };
        match (p.base_local(), p.projection.as_slice()) {
            (Some(l), [Projection::Index(_) | Projection::IndexPlus(..) | Projection::ConstantIndex { .. }]) => {
                self.tables.get(&l).copied()
            }
            _ => None,
        }
    }

    /// The range of a linear combination, `Lin(+3*copy _1, -2*copy _2)`.
    fn lin(&self, facts: &Facts, terms: &[(i64, Operand)]) -> Option<Range> {
        terms.iter().try_fold([0, 0], |sum, (k, o)| {
            let r = self.val(facts, o)?;
            Some(add(sum, arange(BinOp::Mul, r, [*k, *k])?))
        })
    }

    /// One block, from what holds on the way in: what holds on the way to each successor. With `rewrite`, the
    /// checks the ranges prove are dropped.
    fn visit(&mut self, b: &mut Block, mut facts: Facts, rewrite: bool) -> Edges {
        let mut bools: HashMap<Local, Comparison> = HashMap::new(); // bool local -> the comparison it holds
        let mut alias: HashMap<Local, Local> = HashMap::new(); // x -> y while x = copy y holds
        let names = |o: &Operand, x: Local| o.local() == Some(x) && matches!(o, Operand::Place(_));
        let kill =
            |x: Local, facts: &mut Facts, bools: &mut HashMap<Local, Comparison>, alias: &mut HashMap<Local, Local>| {
                facts.remove(&Key::Local(x));
                bools.remove(&x);
                alias.remove(&x);
                bools.retain(|_, (_, a, c)| !names(a, x) && !names(c, x));
                alias.retain(|_, y| *y != x);
            };
        // a plain op's result past a card, as the statement after it may read it, and as its cell keeps it once
        // that statement is done: the local, the two ranges, and whether that statement has begun
        let mut fresh: Option<(Local, Range, Range, bool)> = None;
        let settle = |facts: &mut Facts, (x, read, kept, _): (Local, Range, Range, bool)| {
            if facts.get(&Key::Local(x)) == Some(&read) {
                facts.insert(Key::Local(x), kept);
            }
        };
        let mut i = 0;
        while i < b.statements.len() {
            let last = i + 1 == b.statements.len();
            let s = b.statements[i].clone();
            if s != Statement::Nop {
                match fresh {
                    Some(f) if f.3 => {
                        settle(&mut facts, f);
                        fresh = None;
                    }
                    Some(ref mut f) => f.3 = true,
                    None => {}
                }
            }
            if let (true, Statement::Assign(_, rv)) = (self.judged.is_some(), &s) {
                let r = match rv {
                    Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), a, c) => Some(
                        self.val(&facts, a).zip(self.val(&facts, c)).and_then(|(ra, rb)| arange(*op, ra, rb)),
                    ),
                    Rvalue::Lin(terms) => Some(self.lin(&facts, terms)),
                    _ => None,
                };
                if let Some(r) = r {
                    let past = !r.is_some_and(|r| r[0] >= -999 && r[1] <= 999);
                    self.judged.as_mut().unwrap().push((b.name, i, s.clone(), past));
                }
            }
            // an element of a constant table: the range of its values
            if let Some((x, Rvalue::Use(o))) = assigned_local(&s) {
                if let Some(r) = self.element(o) {
                    kill(x, &mut facts, &mut bools, &mut alias);
                    if !self.taken.contains(&x) {
                        facts.insert(Key::Local(x), r);
                    }
                    i += 1;
                    continue;
                }
            }
            if let Some((x, Rvalue::Use(o))) = assigned_local(&s) {
                if o.int().is_some() || o.local().is_some_and(|l| l != Local::UNIT) {
                    let r = self.val(&facts, o);
                    kill(x, &mut facts, &mut bools, &mut alias);
                    if let Some(r) = r {
                        if !self.taken.contains(&x) {
                            facts.insert(Key::Local(x), r);
                        }
                    }
                    if let Some(y) = o.local().filter(|y| !self.taken.contains(y)) {
                        let to = alias.get(&y).copied().unwrap_or(y);
                        alias.insert(x, to);
                    }
                    i += 1;
                    continue;
                }
            }
            if let Some((x, Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow), a, c))) =
                assigned_local(&s)
            {
                let (ra, rb) = (self.val(&facts, a), self.val(&facts, c));
                let w = pair_value(self.locals.get(&x));
                kill(x, &mut facts, &mut bools, &mut alias);
                if let (Some(ra), Some(rb), Some(w)) = (ra, rb, w) {
                    let adding = *op == BinOp::AddWithOverflow;
                    let r = if adding { add(ra, rb) } else { sub(ra, rb) };
                    let fits = type_range(&w).is_some_and(|[lo, hi]| r[0] >= lo && r[1] <= hi);
                    let t = overflow_assert(&b.terminator);
                    if fits && t.is_some_and(|t| t.0 == x) && last {
                        if rewrite {
                            let plain = if adding { BinOp::Add } else { BinOp::Sub };
                            let field = Place {
                                base: Place::local(x).base,
                                projection: vec![Projection::Field(0, w)],
                            };
                            b.statements[i] = Statement::Assign(field, Rvalue::BinaryOp(plain, a.clone(), c.clone()));
                            b.terminator = Terminator::Goto(t.unwrap().1);
                            self.dropped += 1;
                        }
                        facts.insert(Key::Value(x), r);
                    } else if rewrite && last {
                        self.drop_unread(b, i, x, r, &w);
                    }
                }
                i += 1;
                continue;
            }
            if let Some((x, Rvalue::Use(Operand::Move(p) | Operand::Copy(p)))) = assigned_local(&s) {
                if let ([Projection::Field(0, t)], Some(y)) = (p.projection.as_slice(), p.base_local()) {
                    if one_word(t) {
                        let r = facts.get(&Key::Value(y)).copied();
                        kill(x, &mut facts, &mut bools, &mut alias);
                        if let Some(r) = r {
                            if !self.taken.contains(&x) {
                                facts.insert(Key::Local(x), r);
                            }
                        }
                        i += 1;
                        continue;
                    }
                }
            }
            if let Some((x, Rvalue::BinaryOp(BinOp::BitAnd, a, c))) = assigned_local(&s) {
                if a.local().is_some() || c.local().is_some() {
                    // a & c with c not below 0 is not below 0 and not above c, whatever a is (`x & 3` is
                    // 0..=3, not the 0 or 1 of two bools)
                    let (ra, rc) = (self.val(&facts, a), self.val(&facts, c));
                    let hi = [ra, rc].into_iter().flatten().filter(|r| r[0] >= 0).map(|r| r[1]).min();
                    kill(x, &mut facts, &mut bools, &mut alias);
                    if let Some(hi) = hi {
                        facts.insert(Key::Local(x), [0, hi]);
                    }
                    i += 1;
                    continue;
                }
            }
            if let Some((x, Rvalue::BinaryOp(op, a, c))) = assigned_local(&s) {
                if is_comparison(*op) {
                    let op = *op;
                    kill(x, &mut facts, &mut bools, &mut alias);
                    bools.insert(x, (op, bare(a), bare(c)));
                    // x == c when x's range leaves c out: known false
                    let kc = int_value(c);
                    let rx = self.val(&facts, a);
                    if op == BinOp::Eq && matches!((kc, rx), (Some(k), Some(r)) if k < r[0] || k > r[1]) {
                        facts.insert(Key::Local(x), [0, 0]);
                    }
                    // an order comparison the ranges settle: known true or false. A bounds check the index's range
                    // proves is one: the assert on it goes below, and dead_code then drops the comparison.
                    // (a usize below 0 is one past a card, above every other: a signed order of it would be wrong)
                    let past = |o: &Operand, r: Range| r[0] < 0 && o.local().is_some_and(|l| self.huge.contains(&l));
                    if let (Some(ra), Some(rb)) = (self.val(&facts, a), self.val(&facts, c)) {
                        if !self.taken.contains(&x) && !past(a, ra) && !past(c, rb) {
                            let (yes, no) = match op {
                                BinOp::Lt => (ra[1] < rb[0], ra[0] >= rb[1]),
                                BinOp::Le => (ra[1] <= rb[0], ra[0] > rb[1]),
                                BinOp::Gt => (ra[0] > rb[1], ra[1] <= rb[0]),
                                BinOp::Ge => (ra[0] >= rb[1], ra[1] < rb[0]),
                                _ => (false, false),
                            };
                            if yes {
                                facts.insert(Key::Local(x), [1, 1]);
                            } else if no {
                                facts.insert(Key::Local(x), [0, 0]);
                            }
                        }
                    }
                    i += 1;
                    continue;
                }
            }
            if let Statement::Assign(
                d,
                Rvalue::BinaryOp(op @ (BinOp::AddWrapping | BinOp::SubWrapping | BinOp::MulWrapping), a, c),
            ) = &s
            {
                // a wrapping op: when its result stays within its type's range it is the plain op, which wraps
                // nothing; else it wraps, and its value is anywhere in the type's range
                let plain = unwrapped(*op);
                let r = match (self.val(&facts, a), self.val(&facts, c)) {
                    (Some(ra), Some(rb)) => arange(plain, ra, rb),
                    _ => None,
                };
                let fits = match (r, place_ty(self.locals, d).as_ref().and_then(type_range)) {
                    (Some(r), Some([lo, hi])) => r[0] >= lo && r[1] <= hi,
                    _ => false,
                };
                if fits && rewrite {
                    b.statements[i] = Statement::Assign(d.clone(), Rvalue::BinaryOp(plain, a.clone(), c.clone()));
                }
                if let Some(x) = killed(&s) {
                    kill(x, &mut facts, &mut bools, &mut alias);
                }
                let into = d.as_local().filter(|l| *l != Local::UNIT);
                if let (Some(x), false) = (into, fits) {
                    if self.locals.get(&x) == Some(&Ty::Int(IntTy::Usize)) {
                        self.below.insert(x);
                    }
                }
                if let (Some(x), Some(r)) = (into, r) {
                    // a usize below 0 is held as the negative number it is
                    let held = fits || (self.huge.contains(&x) && r[0] >= -999 && r[1] <= 999);
                    if held && !self.taken.contains(&x) {
                        facts.insert(Key::Local(x), r);
                    }
                }
                i += 1;
                continue;
            }
            if let Some((
                x,
                Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem), a, c),
            )) = assigned_local(&s)
            {
                // unchecked, or checked ones proven above
                let op = *op;
                let (ra, rb) = (self.val(&facts, a), self.val(&facts, c));
                kill(x, &mut facts, &mut bools, &mut alias);
                // dividing a value known not to be negative by a positive constant: the short unsigned loop will do
                if rewrite
                    && matches!(op, BinOp::Div | BinOp::Rem)
                    && ra.is_some_and(|r| r[0] >= 0)
                    && rb.is_some_and(|r| r[0] == r[1] && r[0] > 0)
                    && self.locals.get(&x) == Some(&Ty::Int(IntTy::I16))
                {
                    let unsigned = if op == BinOp::Div { BinOp::UDiv } else { BinOp::URem };
                    b.statements[i] =
                        Statement::Assign(Place::local(x), Rvalue::BinaryOp(unsigned, a.clone(), c.clone()));
                }
                if let (Some(ra), Some(rb)) = (ra, rb) {
                    if let Some(r) = arange(op, ra, rb) {
                        if !self.taken.contains(&x) {
                            if let Some(f) = fresh.take().filter(|f| f.3 && f.0 != x) {
                                settle(&mut facts, f); // this statement read it
                            }
                            facts.insert(Key::Local(x), kept(r));
                            fresh = Some((x, kept(r), stored(r), false));
                        }
                    }
                }
                i += 1;
                continue;
            }
            if let Some((x, Rvalue::Lin(terms))) = assigned_local(&s) {
                let r = self.lin(&facts, terms);
                kill(x, &mut facts, &mut bools, &mut alias);
                if let (Some(r), false) = (r, self.taken.contains(&x)) {
                    if let Some(f) = fresh.take().filter(|f| f.3 && f.0 != x) {
                        settle(&mut facts, f);
                    }
                    facts.insert(Key::Local(x), kept(r));
                    fresh = Some((x, kept(r), stored(r), false));
                }
                i += 1;
                continue;
            }
            if let Some((x, Rvalue::BinaryOp(BinOp::MulWithOverflow, a, c))) = assigned_local(&s) {
                let (ra, rb) = (self.val(&facts, a), self.val(&facts, c));
                let w = pair_value(self.locals.get(&x));
                kill(x, &mut facts, &mut bools, &mut alias);
                let r = match (ra, rb) {
                    (Some(ra), Some(rb)) => arange(BinOp::Mul, ra, rb),
                    _ => None,
                };
                let t = overflow_assert(&b.terminator);
                if let (Some(r), Some(w)) = (r, w) {
                    let fits = type_range(&w).is_some_and(|[lo, hi]| r[0] >= lo && r[1] <= hi);
                    if fits && t.is_some_and(|t| t.0 == x) && last {
                        if rewrite {
                            let field = Place {
                                base: Place::local(x).base,
                                projection: vec![Projection::Field(0, w)],
                            };
                            b.statements[i] =
                                Statement::Assign(field, Rvalue::BinaryOp(BinOp::Mul, a.clone(), c.clone()));
                            b.terminator = Terminator::Goto(t.unwrap().1);
                            self.dropped += 1;
                        }
                        facts.insert(Key::Value(x), r);
                    } else if rewrite && last {
                        self.drop_unread(b, i, x, r, &w);
                    }
                }
                i += 1;
                continue;
            }
            if let Some((x, Rvalue::Cast(o @ (Operand::Copy(_) | Operand::Move(_)), to, kind))) = assigned_local(&s) {
                if let (Some(y), true, true) = (o.local(), *kind == CastKind::IntToInt, one_word(to)) {
                    let r = self.val(&facts, o);
                    kill(x, &mut facts, &mut bools, &mut alias);
                    if let (Some(r), Some(t)) = (r, type_range(to)) {
                        if r[0] >= t[0] && r[1] <= t[1] {
                            // known not negative (and for a u8, below 256): no check, nothing to wrap
                            if rewrite
                                && self.locals.get(&y) == Some(&Ty::Int(IntTy::I16))
                                && matches!(to, Ty::Int(IntTy::Usize | IntTy::U8))
                            {
                                b.statements[i] =
                                    Statement::Assign(Place::local(x), Rvalue::Use(Operand::Copy(Place::local(y))));
                                self.dropped += 1;
                            }
                            if !self.taken.contains(&x) {
                                facts.insert(Key::Local(x), r);
                            }
                        } else if self.huge.contains(&x) && r[0] >= -999 && r[1] <= 999 {
                            // a usize past a card, held as the negative number it is
                            facts.insert(Key::Local(x), r);
                        }
                    }
                    i += 1;
                    continue;
                }
            }
            if let Some((x, Rvalue::UnaryOp(UnOp::Neg, a))) = assigned_local(&s) {
                // -a: on a card, never past -999..999 (no i16::MIN)
                let r = self.val(&facts, a);
                kill(x, &mut facts, &mut bools, &mut alias);
                if let (Some([lo, hi]), false) = (r, self.taken.contains(&x)) {
                    facts.insert(Key::Local(x), [-hi, -lo]);
                }
                i += 1;
                continue;
            }
            if let Some(x) = killed(&s) {
                kill(x, &mut facts, &mut bools, &mut alias);
            }
            i += 1;
        }
        if let Some(f) = fresh {
            settle(&mut facts, f);
        }
        // what the terminator tells each successor
        // an assert of something known true always passes
        if let Some((x, true, target)) = assert_on_local(&b.terminator) {
            if self.val(&facts, &Operand::Copy(Place::local(x))).unwrap_or([0, 0])[0] == 1 && rewrite {
                b.terminator = Terminator::Goto(target);
                self.dropped += 1;
            }
        }
        // an assert of something known false, negated, always passes (rustc's divide-by-zero and overflow checks)
        if let Some((x, false, target)) = assert_on_local(&b.terminator) {
            if self.val(&facts, &Operand::Copy(Place::local(x))).unwrap_or([1, 1])[1] == 0 && rewrite {
                b.terminator = Terminator::Goto(target);
                self.dropped += 1;
            }
        }
        // a test the ranges decide, `if x >= 0` with x a counter from 0: one way, no test; only for an i16 or a
        // u8, whose range is what it holds (a usize past a card is held below 0)
        if rewrite {
            let mut decided = None;
            if let Terminator::SwitchInt {
                discr: Operand::Move(p) | Operand::Copy(p),
                arms,
                otherwise: Some(yes),
            } = &b.terminator
            {
                if let (Some(x), [(0, no)]) = (p.as_local(), arms.as_slice()) {
                    if let Some((op, a, c)) = bools.get(&x) {
                        // k op x, as rustc writes a range pattern's start (`6 <= y`): x op' k
                        let (op, a, c) = match (int_value(a), int_value(c)) {
                            (Some(_), None) => {
                                let flipped = match op {
                                    BinOp::Lt => BinOp::Gt,
                                    BinOp::Le => BinOp::Ge,
                                    BinOp::Gt => BinOp::Lt,
                                    BinOp::Ge => BinOp::Le,
                                    op => *op,
                                };
                                (flipped, c, a)
                            }
                            _ => (*op, a, c),
                        };
                        let card = a
                            .local()
                            .is_some_and(|l| matches!(self.locals.get(&l), Some(Ty::Int(IntTy::I16 | IntTy::U8))));
                        if let (true, Some(v), Some(r)) = (card, int_value(c), self.val(&facts, a)) {
                            let within = |s: Option<Range>| s.is_some_and(|s| r[0] >= s[0] && r[1] <= s[1]);
                            if within(held(op, v)) {
                                decided = Some(*yes);
                            } else if within(not_held(op, v)) {
                                decided = Some(*no);
                            }
                        }
                    }
                }
            }
            if let Some(to) = decided {
                b.terminator = Terminator::Goto(to);
                self.dropped += 1;
            }
        }
        let mut edge = Edges::new();
        for s in b.terminator.successors() {
            edge.insert(s, facts.clone());
        }
        if let Terminator::Call {
            dest,
            func,
            args,
            target: Some(t),
            ..
        } = &b.terminator
        {
            // what a call gives is unknown, but for digit(v, k), a digit: 0 to 9, less than 0 only for a v that is
            let digit = match args.as_slice() {
                [v, _] if func.is_path(&["digit"]) => Some(match self.val(&facts, v) {
                    Some([lo, _]) if lo >= 0 => [0, 9],
                    _ => [-9, 9],
                }),
                _ => None,
            };
            if let (Some(x), Some(e)) = (dest.as_local().filter(|l| *l != Local::UNIT), edge.get_mut(t)) {
                match digit {
                    Some(r) => e.insert(Key::Local(x), r),
                    None => e.remove(&Key::Local(x)),
                };
            }
        }
        let narrow1 = |edge: &mut Edges, target: BasicBlock, x: Local, lo: i64, hi: i64| {
            if !edge.contains_key(&target) || self.taken.contains(&x) {
                return;
            }
            let e = edge.get_mut(&target).unwrap();
            if let Some(r) = e.get(&Key::Local(x)).copied().or_else(|| self.of_type(Key::Local(x))) {
                let mut lo = lo;
                if self.locals.get(&x) == Some(&Ty::Int(IntTy::Usize)) {
                    // compared as a usize: one below a bound is not below 0, and one above it may be a usize past
                    // a card, held below 0
                    if lo <= -BIG {
                        lo = 0;
                    } else if hi >= BIG && r[0] < 0 && self.huge.contains(&x) {
                        return;
                    }
                }
                e.insert(Key::Local(x), [r[0].max(lo), r[1].min(hi)]);
            }
        };
        let narrow = |edge: &mut Edges, target: BasicBlock, x: &Operand, r: Range| {
            let Some(x) = x.local().filter(|_| matches!(x, Operand::Place(_))) else {
                return;
            };
            narrow1(edge, target, x, r[0], r[1]);
            if let Some(y) = alias.get(&x) {
                narrow1(edge, target, *y, r[0], r[1]);
            }
        };
        // an assert that passed: the bool was what it asked for, from here on (rustc tests one zero check twice)
        if let Some((x, expected, target)) = assert_on_local(&b.terminator) {
            if edge.contains_key(&target) && !self.taken.contains(&x) {
                edge.get_mut(&target)
                    .unwrap()
                    .insert(Key::Local(x), if expected { [1, 1] } else { [0, 0] });
            }
        }
        // an assert that passed: its comparison held (or, negated, did not); a copy of the bool too, as GVN asserts
        // one `r < 6` again
        if let Terminator::Assert {
            cond: Operand::Move(p) | Operand::Copy(p),
            expected,
            target,
            unwind,
            ..
        } = &b.terminator
        {
            if let (Some(x), true) = (p.as_local(), plain_unwind(unwind)) {
                if let Some((op, a, c)) = bools.get(&x) {
                    if let Some(v) = int_value(c) {
                        let r = if *expected { held(*op, v) } else { not_held(*op, v) };
                        if let Some(r) = r {
                            narrow(&mut edge, *target, a, r);
                        }
                    }
                }
            }
        }
        if let Terminator::SwitchInt {
            discr: Operand::Move(p),
            arms,
            otherwise: Some(yes),
        } = &b.terminator
        {
            if let (Some(x), [(0, no)]) = (p.as_local(), arms.as_slice()) {
                if let Some((op, a, c)) = bools.get(&x) {
                    if let Some(v) = int_value(c) {
                        // x op v
                        if let (Some(tr), Some(fl)) = (held(*op, v), not_held(*op, v)) {
                            narrow(&mut edge, *yes, a, tr);
                            narrow(&mut edge, *no, a, fl);
                        }
                    } else if let Some(v) = int_value(a) {
                        // v op x
                        let (tr, fl) = match op {
                            BinOp::Lt => (Some([v.saturating_add(1), BIG]), Some([-BIG, v])),
                            BinOp::Le => (Some([v, BIG]), Some([-BIG, v.saturating_sub(1)])),
                            BinOp::Gt => (Some([-BIG, v.saturating_sub(1)]), Some([v, BIG])),
                            BinOp::Ge => (Some([-BIG, v]), Some([v.saturating_add(1), BIG])),
                            _ => (None, None),
                        };
                        if let (Some(tr), Some(fl)) = (tr, fl) {
                            narrow(&mut edge, *yes, c, tr);
                            narrow(&mut edge, *no, c, fl);
                        }
                    } else if let (true, Some(ra), Some(rc)) = (
                        // x op y, two cards: each is bounded by the other's range on each way (`end >= start`, start
                        // 0 or more, is end 0 or more); a usize too, where it can't be past a card (`i < r + 3` in a
                        // loop over a range)
                        [a, c].iter().all(|o| {
                            o.local().is_some_and(|l| match self.locals.get(&l) {
                                Some(Ty::Int(IntTy::I16 | IntTy::U8)) => true,
                                Some(Ty::Int(IntTy::Usize)) => !self.huge.contains(&l),
                                _ => false,
                            })
                        }),
                        self.val(&facts, a),
                        self.val(&facts, c),
                    ) {
                        // (a's range, c's range) where a op c holds, and where it doesn't
                        let at_least = |r: Range, by: i64| [r[0].saturating_add(by), BIG];
                        let at_most = |r: Range, by: i64| [-BIG, r[1].saturating_sub(by)];
                        let ways = match op {
                            BinOp::Lt => Some(((at_most(rc, 1), at_least(ra, 1)), (at_least(rc, 0), at_most(ra, 0)))),
                            BinOp::Le => Some(((at_most(rc, 0), at_least(ra, 0)), (at_least(rc, 1), at_most(ra, 1)))),
                            BinOp::Gt => Some(((at_least(rc, 1), at_most(ra, 1)), (at_most(rc, 0), at_least(ra, 0)))),
                            BinOp::Ge => Some(((at_least(rc, 0), at_most(ra, 0)), (at_most(rc, 1), at_least(ra, 1)))),
                            _ => None,
                        };
                        if let Some(((ta, tc), (fa, fc))) = ways {
                            narrow(&mut edge, *yes, a, ta);
                            narrow(&mut edge, *yes, c, tc);
                            narrow(&mut edge, *no, a, fa);
                            narrow(&mut edge, *no, c, fc);
                        }
                    }
                }
            }
        }
        // a switch on an integer's value, `switchInt(copy n) -> [0: done, otherwise: step]`: on the arm's way n is its
        // value, and on the other way it is not, which narrows a range that ends there (a u8 counted down to 0)
        if let Terminator::SwitchInt {
            discr: Operand::Move(p) | Operand::Copy(p),
            arms,
            otherwise: Some(other),
        } = &b.terminator
        {
            let int = p.as_local().filter(|x| matches!(self.locals.get(x), Some(Ty::Int(_))));
            // (a negative arm is written as its two's complement: only a value a card holds as it is)
            if let (Some(x), [(v @ 0..=999, arm)]) = (int, arms.as_slice()) {
                let v = *v as i64;
                let x = Operand::Place(Place::local(x));
                narrow(&mut edge, *arm, &x, [v, v]);
                let known = self.val(&facts, &x);
                if let (Some(r), true) = (known, arm != other) {
                    if r[0] == v {
                        narrow(&mut edge, *other, &x, [v + 1, r[1]]);
                    } else if r[1] == v {
                        narrow(&mut edge, *other, &x, [r[0], v - 1]);
                    }
                }
            }
        }
        // a promise (assert_unchecked) has given its facts: no code
        if rewrite {
            if let Terminator::Assert {
                cond: Operand::Move(p),
                msg,
                msg_args,
                target,
                ..
            } = &b.terminator
            {
                if p.as_local().is_some() && msg == "assumed" && msg_args.is_empty() {
                    b.terminator = Terminator::Goto(*target);
                    self.dropped += 1;
                }
            }
        }
        edge
    }
}

/// The arrays set once, whole, from integer constants (`[0, 1, 4, 9]`, `[0; 8]`), and otherwise only by element to
/// an integer constant (`b[i] = 1`): not by a call, not a parameter (nothing sets one in the body), and with no
/// address taken. Every element read is then one of those constants: the range from the least to the greatest.
pub fn constant_tables(blocks: &[Block], taken: &HashSet<Local>) -> HashMap<Local, Range> {
    // for each array: its whole sets, and its element sets, each the range of its constants (none: not constants)
    let mut whole: HashMap<Local, Vec<Option<Range>>> = HashMap::new();
    let mut elements: HashMap<Local, Vec<Option<Range>>> = HashMap::new();
    let constants = |os: &mut dyn Iterator<Item = &Operand>| -> Option<Range> {
        let mut r: Option<Range> = None;
        for o in os {
            let v = int_value(o)?;
            r = Some(match r {
                None => [v, v],
                Some([lo, hi]) => [lo.min(v), hi.max(v)],
            });
        }
        r
    };
    for b in blocks {
        for s in &b.statements {
            if let Statement::Assign(d, rv) = s {
                let Some(l) = d.base_local() else { continue };
                let element = matches!(
                    d.projection.as_slice(),
                    [Projection::Index(_) | Projection::IndexPlus(..) | Projection::ConstantIndex { .. }]
                );
                match (d.projection.is_empty(), element, rv) {
                    (true, _, Rvalue::Array(os)) => whole.entry(l).or_default().push(constants(&mut os.iter())),
                    (true, _, Rvalue::Repeat(o, _)) => {
                        whole.entry(l).or_default().push(constants(&mut std::iter::once(o)))
                    }
                    (true, ..) => whole.entry(l).or_default().push(None),
                    (false, true, Rvalue::Use(o)) => {
                        elements.entry(l).or_default().push(constants(&mut std::iter::once(o)))
                    }
                    _ => elements.entry(l).or_default().push(None),
                }
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            if let Some(l) = dest.base_local() {
                whole.entry(l).or_default().push(None);
            }
        }
    }
    let hull = |a: Range, b: Range| [a[0].min(b[0]), a[1].max(b[1])];
    whole
        .into_iter()
        .filter(|(l, _)| !taken.contains(l))
        .filter_map(|(l, rs)| {
            let [Some(r)] = rs.as_slice() else { return None };
            elements
                .get(&l)
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .try_fold(*r, |acc, e| e.map(|e| hull(acc, e)))
                .map(|r| (l, r))
        })
        .collect()
}

/// The locals whose address is taken as a whole or at an index: `&x`, `&mut x`, `&raw mut x[0 of 1]`.
pub fn taken_locals(blocks: &[Block]) -> HashSet<Local> {
    let mut taken = HashSet::new();
    for b in blocks {
        for s in &b.statements {
            if let Statement::Assign(_, Rvalue::Ref(_, p)) = s {
                let indexed = p.projection.iter().all(|x| {
                    matches!(
                        x,
                        Projection::Index(_) | Projection::IndexPlus(..) | Projection::ConstantIndex { .. }
                    )
                });
                if let (Some(l), true) = (p.base_local(), indexed) {
                    taken.insert(l);
                }
            }
        }
    }
    taken
}

/// The overflow-checked pairs whose value nothing reads: every line naming one either assigns the whole pair (its
/// op) or is its overflow check. (dead_code runs first, so a copy of the value nobody reads is gone already.)
fn unread_pairs(blocks: &[Block], locals: &HashMap<Local, Ty>, taken: &HashSet<Local>) -> HashSet<Local> {
    let mut unread: HashSet<Local> = locals
        .iter()
        .filter(|(l, t)| pair_value(Some(t)).is_some() && !taken.contains(l))
        .map(|(l, _)| *l)
        .collect();
    for b in blocks {
        for s in &b.statements {
            let whole = assigned_local(s).map(|(l, _)| l);
            for l in s.locals() {
                if Some(l) != whole {
                    unread.remove(&l);
                }
            }
        }
        let check = overflow_assert(&b.terminator).map(|(l, _)| l);
        for l in b.terminator.locals() {
            if Some(l) != check {
                unread.remove(&l);
            }
        }
    }
    unread
}

/// Run the range pass over a function's blocks, dropping the checks it proves. Returns how many it dropped, and the
/// plain sums, differences and products (and linear combinations) it finds stay on a card, each written once in
/// the function: `past_a_card` takes their word, as by lowering the facts that showed it may be gone (an
/// `assert_unchecked` promise, say).
pub fn range_pass(
    blocks: &mut [Block],
    locals: &HashMap<Local, Ty>,
    promised: &HashSet<Local>,
) -> Result<(usize, HashSet<Statement>), Fail> {
    let (dropped, judged) = analyse(blocks, locals, promised, false)?;
    Ok((dropped, on_a_card(&judged)))
}

/// The statements judged to stay on a card, of those written once.
fn on_a_card(judged: &[(BasicBlock, usize, Statement, bool)]) -> HashSet<Statement> {
    let mut times: HashMap<&Statement, usize> = HashMap::new();
    for (_, _, s, _) in judged {
        *times.entry(s).or_default() += 1;
    }
    judged
        .iter()
        .filter(|(_, _, s, past)| !past && times[s] == 1)
        .map(|(_, _, s, _)| s.clone())
        .collect()
}

/// The plain sums, differences and products, and the linear combinations, in a function's blocks as they are
/// lowered, whose result may be past a card and is read more than once, by block and statement index: the
/// accumulator holds more of it than its cell keeps, and can't stand in for the cell after the store (see
/// `Line::wide`), or one reading would see 1200 and another 200. A local read in one place only is read once for
/// each value it is given, and may be read exact there, as a carried one is (see `carries` in lowering). The range
/// pass's analysis, read only; a function it can't settle has every one.
pub fn past_a_card(
    blocks: &[Block],
    locals: &HashMap<Local, Ty>,
    on_card: &HashSet<Statement>,
    promised: &HashSet<Local>,
) -> HashSet<(BasicBlock, usize)> {
    let mut copy = blocks.to_vec();
    let past: HashSet<(BasicBlock, usize)> = match analyse(&mut copy, locals, promised, true) {
        Ok((_, judged)) => {
            // (one the range pass found stays on a card, written once then and once now)
            let now = on_a_card(&judged.iter().map(|(b, i, s, _)| (*b, *i, s.clone(), false)).collect::<Vec<_>>());
            judged
                .into_iter()
                .filter(|(_, _, s, past)| *past && !(on_card.contains(s) && now.contains(s)))
                .map(|(b, i, _, _)| (b, i))
                .collect()
        }
        Err(_) => blocks
            .iter()
            .flat_map(|b| (0..b.statements.len()).map(move |i| (b.name, i)))
            .collect(),
    };
    // how many times each local is read: every time it is named, less each time it is assigned as a whole
    let mut reads: HashMap<Local, i64> = HashMap::new();
    for b in blocks {
        for l in b.lines() {
            l.visit_locals(&mut |x| *reads.entry(x).or_default() += 1);
        }
        for s in &b.statements {
            if let Some((x, _)) = assigned_local(s) {
                *reads.entry(x).or_default() -= 1;
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            if let Some(x) = dest.as_local() {
                *reads.entry(x).or_default() -= 1;
            }
        }
    }
    let read_once = |&(name, i): &(BasicBlock, usize)| -> bool {
        let x = blocks
            .iter()
            .find(|b| b.name == name)
            .and_then(|b| b.statements.get(i))
            .and_then(assigned_local)
            .map(|(x, _)| x);
        x.is_some_and(|x| reads.get(&x).is_some_and(|&n| n <= 1))
    };
    past.into_iter().filter(|p| !read_once(p)).collect()
}

/// The range pass, or with `record` only what `past_a_card` finds, changing nothing.
fn analyse(
    blocks: &mut [Block],
    locals: &HashMap<Local, Ty>,
    promised: &HashSet<Local>,
    record: bool,
) -> Result<(usize, Vec<(BasicBlock, usize, Statement, bool)>), Fail> {
    let taken = taken_locals(blocks);
    let preds = predecessors(blocks);
    let unread = unread_pairs(blocks, locals, &taken);
    let mut pass = Pass {
        locals,
        taken,
        unread,
        huge: HashSet::new(),
        below: HashSet::new(),
        dropped: 0,
        tables: constant_tables(blocks, &taken_locals(blocks)),
        judged: None,
    };
    let mut out: HashMap<BasicBlock, Edges> = HashMap::new(); // block -> what holds on the way to each successor
                                                              // the program's own constants, each with its neighbours: where a growing range is widened to
    let steps: Vec<i64> = {
        let mut seen: Vec<i64> = vec![];
        for b in blocks.iter() {
            let operands = b
                .statements
                .iter()
                .flat_map(|s| s.operands())
                .chain(b.terminator.operands());
            for c in operands.filter_map(int_value) {
                if !seen.contains(&c) {
                    seen.push(c);
                }
            }
        }
        seen.iter()
            .flat_map(|&c| [c.saturating_sub(1), c, c.saturating_add(1)])
            .collect()
    };
    // What holds on the way into a block: on every way in seen so far (one not yet reached is ignored, so loops
    // start from what holds on entry), the hull of the ranges. Iterated to a fixed point, widening a range that
    // keeps moving; then a last visit drops the checks the ranges prove.
    // the blocks the entry reaches: one it does not never runs, and gives nothing to the blocks it would jump to
    let reached: HashSet<BasicBlock> = {
        let at: HashMap<BasicBlock, &Block> = blocks.iter().map(|b| (b.name, b)).collect();
        let mut seen = HashSet::new();
        let mut todo: Vec<BasicBlock> = blocks.first().map(|b| b.name).into_iter().collect();
        while let Some(n) = todo.pop() {
            if seen.insert(n) {
                todo.extend(at.get(&n).into_iter().flat_map(|b| b.terminator.successors()));
            }
        }
        seen
    };
    let in_of = |pass: &Pass,
                 out: &HashMap<BasicBlock, Edges>,
                 b: &Block,
                 prev: Option<&Facts>,
                 iter: usize,
                 settled: bool|
     -> Option<Facts> {
        let ins: Vec<&Facts> = preds
            .get(&b.name)
            .into_iter()
            .flatten()
            .filter(|p| reached.contains(p))
            .filter_map(|p| out.get(p).and_then(|o| o.get(&b.name)))
            .collect();
        // a way in where some range is empty is never taken (`233 <= x` after `x < 233`): in the rounds of
        // narrowing, from settled facts (before, a range may be empty only for now), and only while another way in
        // can be taken (a block none can reach keeps what its ways say, which settles its checks)
        let feasible = |o: &&Facts| o.values().all(|r| r[0] <= r[1]);
        let ins: Vec<&Facts> = if settled && ins.iter().any(feasible) {
            ins.into_iter().filter(feasible).collect()
        } else {
            ins
        };
        if ins.is_empty() {
            return None;
        }
        let mut f = Facts::new();
        let keys: HashSet<Key> = ins.iter().flat_map(|o| o.keys().copied()).collect();
        for x in keys {
            let whole = pass.of_type(x);
            let rs: Vec<Option<Range>> = ins.iter().map(|o| o.get(&x).copied().or(whole)).collect();
            if rs.iter().any(|r| r.is_none()) {
                continue;
            }
            let rs: Vec<Range> = rs.into_iter().map(|r| r.unwrap()).collect();
            let mut r = [
                rs.iter().map(|q| q[0]).min().unwrap(),
                rs.iter().map(|q| q[1]).max().unwrap(),
            ];
            // still growing after a few rounds: widen out to the next of the program's own constants (a loop's
            // bound, say), or the type's range, and never shrink back (the rounds of narrowing after do that). A
            // bound that steps through constant after constant, round after round, goes to the type's end at once.
            if iter > 6 {
                let stepping = iter <= STEPPING;
                if let Some(prev) = prev {
                    let Some(p) = prev.get(&x).copied().or(whole) else {
                        continue;
                    };
                    if r[0] < p[0] {
                        r[0] = steps
                            .iter()
                            .copied()
                            .filter(|&c| stepping && c <= r[0])
                            .fold(whole.map_or(-BIG, |w| w[0]), i64::max);
                    } else {
                        r[0] = p[0];
                    }
                    if r[1] > p[1] {
                        r[1] = steps
                            .iter()
                            .copied()
                            .filter(|&c| stepping && c >= r[1])
                            .fold(whole.map_or(BIG, |w| w[1]), i64::min);
                    } else {
                        r[1] = p[1];
                    }
                }
            }
            f.insert(x, r);
        }
        Some(f)
    };
    // Visit blocks in reverse postorder from the entry (each block after the blocks that lead to it, loops aside),
    // and widen only at loop heads, the blocks a back edge comes into: rustc's text order puts inlined code out of
    // order, and widening anywhere else loses ranges it never needed to.
    let index: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let mut post = vec![];
    let mut seen = HashSet::new();
    fn dfs(
        n: BasicBlock,
        blocks: &[Block],
        index: &HashMap<BasicBlock, usize>,
        seen: &mut HashSet<BasicBlock>,
        post: &mut Vec<usize>,
    ) {
        let Some(&i) = index.get(&n) else { return };
        if !seen.insert(n) {
            return;
        }
        for t in blocks[i].terminator.successors() {
            dfs(t, blocks, index, seen, post);
        }
        post.push(i);
    }
    if !blocks.is_empty() {
        dfs(blocks[0].name, blocks, &index, &mut seen, &mut post);
    }
    let mut rpo: Vec<usize> = post.into_iter().rev().collect();
    rpo.extend((0..blocks.len()).filter(|&i| !seen.contains(&blocks[i].name)));
    let at: HashMap<BasicBlock, usize> = rpo.iter().enumerate().map(|(k, &i)| (blocks[i].name, k)).collect();
    let mut heads = HashSet::new();
    for &i in &rpo {
        for t in blocks[i].terminator.successors() {
            if let Some(&k) = at.get(&t) {
                if k <= at[&blocks[i].name] {
                    heads.insert(t);
                }
            }
        }
    }
    // A usize difference makes a usize past a card when it can go below 0, and its range says whether it can, which
    // depends on which usizes can be past a card. So the analysis starts from none (only casts of an i16), and
    // runs again with each difference it finds can go below 0 counted as one, until it finds none more: then each
    // that is not counted is one it proves stays at 0 or above, from what the others hold (see `huge_usizes`).
    let mut wraps: HashSet<Local> = HashSet::new();
    let mut last_in: HashMap<BasicBlock, Option<Facts>>;
    loop {
        // (a usize a promise keeps on a card is none: `blocks::promised_cards`)
        pass.huge = huge_usizes(blocks, locals, promised, Some(&wraps));
        out.clear();
        last_in = HashMap::new();
        for iter in 0..40 {
            let mut changed = false;
            for &i in &rpo {
                let name = blocks[i].name;
                let prev = last_in.get(&name).cloned().flatten();
                let f = in_of(
                    &pass,
                    &out,
                    &blocks[i],
                    prev.as_ref(),
                    if heads.contains(&name) { iter } else { 0 },
                    false,
                );
                last_in.insert(name, f.clone());
                let e = pass.visit(&mut blocks[i], f.unwrap_or_default(), false);
                if out.get(&name) != Some(&e) {
                    out.insert(name, e);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
            if iter == 39 {
                return Err(crate::codes::internal("range analysis did not settle"));
            }
        }
        // Widening loses what a loop's own test gives back (a counter below 12 is at most 12 after its step): a few
        // rounds without widening, starting from the settled facts, narrow the ranges again (each round stays sound).
        for _round in 0..6 {
            for &i in &rpo {
                let name = blocks[i].name;
                let f = in_of(&pass, &out, &blocks[i], None, 0, true);
                last_in.insert(name, f.clone());
                let e = pass.visit(&mut blocks[i], f.unwrap_or_default(), false);
                out.insert(name, e);
            }
        }
        pass.below.clear();
        for b in blocks.iter_mut() {
            let f = last_in.get(&b.name).cloned().flatten().unwrap_or_default();
            pass.visit(b, f, false);
        }
        if pass.below.is_subset(&wraps) {
            break;
        }
        wraps.extend(pass.below.iter().copied());
    }
    pass.judged = Some(vec![]);
    for b in blocks.iter_mut() {
        let f = last_in.get(&b.name).cloned().flatten().unwrap_or_default();
        pass.visit(b, f, !record);
    }
    Ok((pass.dropped, pass.judged.unwrap_or_default()))
}

/// The plain op a wrapping one does when nothing wraps.
pub fn unwrapped(op: BinOp) -> BinOp {
    match op {
        BinOp::AddWrapping => BinOp::Add,
        BinOp::SubWrapping => BinOp::Sub,
        BinOp::MulWrapping => BinOp::Mul,
        op => op,
    }
}

/// The type of a place, from its local's through its projections, where the IR says.
pub(super) fn place_ty(locals: &HashMap<Local, Ty>, p: &Place) -> Option<Ty> {
    let mut t = locals.get(&p.base_local()?)?.clone();
    for x in &p.projection {
        t = match (x, t) {
            (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
            (Projection::Field(_, f), _) => f.clone(),
            (
                Projection::Index(_)
                | Projection::IndexPlus(..)
                | Projection::ConstantIndex { .. }
                | Projection::IndexPlace(_),
                Ty::Array(e, _) | Ty::Slice(e),
            ) => *e,
            _ => return None,
        };
    }
    Some(t)
}

/// Rust's `+`, `-` and `*` on a u8 wrap (the plain ops rustc writes with overflow checks off, and for an inlined
/// `wrapping_add` and its kin), and a card does not: 0 - 1 is -1 in a cell. Each becomes its wrapping op, which
/// lowers with the wrap (mod 256), until the range pass shows that its result stays within 0..=255 and turns it
/// back into the plain op. A usize's `-` likewise: below 0 it wraps to a usize past 999, which a cell holds
/// as the negative number it is (see `huge_usizes`). Run on rustc's MIR before any pass writes plain ops of its own
/// (a for loop's step, `saturating_sub`'s difference, a checked op whose check nothing reads), which wrap nothing.
pub fn mark_wrapping(blocks: &mut [Block], locals: &HashMap<Local, Ty>) {
    let ty = |o: &Operand| match o {
        Operand::Constant(_) => o.int().map(|(_, t)| Ty::Int(t)),
        Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => place_ty(locals, p),
        Operand::Aggregate(..) | Operand::Fn(_) => None,
    };
    for s in blocks.iter_mut().flat_map(|b| b.statements.iter_mut()) {
        let Statement::Assign(d, Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), a, c)) = s else {
            continue;
        };
        let t = place_ty(locals, d);
        if t.is_none() || ty(a) != t || ty(c) != t {
            continue;
        }
        *op = match (t, *op) {
            (Some(Ty::Int(IntTy::U8 | IntTy::I8)), BinOp::Add) => BinOp::AddWrapping,
            (Some(Ty::Int(IntTy::U8 | IntTy::I8 | IntTy::Usize)), BinOp::Sub) => BinOp::SubWrapping,
            (Some(Ty::Int(IntTy::U8 | IntTy::I8)), BinOp::Mul) => BinOp::MulWrapping,
            (_, op) => op,
        };
    }
}

/// The usize locals that can hold a value past what a card holds, as Rust's wrapping gives one: `-1i16 as usize`,
/// or `0usize - 1` without overflow checks, is usize::MAX. A cell holds such a value as the negative number it is
/// short of 2^64 by (-1 for usize::MAX), which is exact for what can be done with it on a card: compared (as the
/// huge number it is: lowering tests the sign first), cast back to i16 (-1) or to u8 (255), indexed with (the bounds
/// check fails), or added to and subtracted from, into another such local. Anything else is past a card, and
/// lowering panics there, as a checked op does with a value past 999. A local of this kind is one that such
/// a cast or difference, or a copy, sum or difference of one of these, is stored into: not one whose address is
/// taken, `_0` or one of `keep_out` (a struct result's fields), which a caller or a pointer reads as a usize a card
/// holds. With `wraps`, only the differences into those locals count (the range pass's own).
pub fn huge_usizes(
    blocks: &[Block],
    locals: &HashMap<Local, Ty>,
    keep_out: &HashSet<Local>,
    wraps: Option<&HashSet<Local>>,
) -> HashSet<Local> {
    let taken = taken_locals(blocks);
    let usize_local = |x: Local| {
        x != Local(0) && !taken.contains(&x) && !keep_out.contains(&x) && locals.get(&x) == Some(&Ty::Int(IntTy::Usize))
    };
    let mut huge: HashSet<Local> = HashSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for s in blocks.iter().flat_map(|b| b.statements.iter()) {
            let Some((x, rv)) = assigned_local(s) else { continue };
            if huge.contains(&x) || !usize_local(x) {
                continue;
            }
            let is_huge = |o: &Operand| {
                o.place()
                    .is_some_and(|p| p.as_local().is_some_and(|l| huge.contains(&l)))
            };
            let from = match rv {
                Rvalue::Cast(o, Ty::Int(IntTy::Usize), CastKind::IntToInt) => {
                    is_huge(o)
                        || o.place()
                            .is_some_and(|p| matches!(place_ty(locals, p), Some(Ty::Int(IntTy::I16 | IntTy::I8))))
                }
                Rvalue::BinaryOp(BinOp::SubWrapping, ..) => wraps.is_none_or(|w| w.contains(&x)),
                Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::AddWrapping, a, c) => is_huge(a) || is_huge(c),
                Rvalue::Use(o) => is_huge(o),
                _ => false,
            };
            if from {
                huge.insert(x);
                changed = true;
            }
        }
    }
    huge
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir::parse_body;

    /// A loop that grows a variable by 998 - k each time round, without overflow checks: its low end creeps
    /// down one round after another through the program's small constants, and the analysis must still settle,
    /// at the type's end. (The compiler drops such a variable first when nothing reads it; this one is read.)
    #[test]
    fn a_range_that_creeps_settles_at_its_type() {
        let body = parse_body(
            "let _1: i16;
            let mut _2: bool;
            let mut _3: bool;
            let mut _5: i16;
            let mut _7: i16;
            let mut _8: bool;
            let mut _10: bool;
            let mut _11: i16;
            let mut _13: i16;
            let _4: ();
            bb0: {
                _1 = cardiac::inp() -> [return: bb1, unwind continue];
            }
            bb1: {
                _3 = Gt(copy _1, const -1_i16);
                _2 = Le(copy _1, const 9_i16);
                _4 = cardiac::out(const 12_i16) -> [return: bb5, unwind continue];
            }
            bb5: {
                _5 = const 15_i16;
                _4 = cardiac::out(const -3_i16) -> [return: bb6, unwind continue];
            }
            bb6: {
                _7 = const 0_i16;
                goto -> bb7;
            }
            bb7: {
                _8 = Lt(copy _7, copy _1);
                switchInt(move _8) -> [0: bb12, otherwise: bb8];
            }
            bb8: {
                _11 = Mul(copy _7, const 4_i16);
                _10 = Le(const 7_i16, move _11);
                switchInt(move _10) -> [0: bb11, otherwise: bb9];
            }
            bb9: {
                _13 = Sub(const 998_i16, copy _7);
                _5 = Add(copy _5, move _13);
                goto -> bb11;
            }
            bb11: {
                _7 = Add(copy _7, const 1_i16);
                goto -> bb7;
            }
            bb12: {
                _4 = cardiac::out(move _5) -> [return: bb13, unwind continue];
            }
            bb13: {
                return;
            }",
        );
        let locals: HashMap<Local, Ty> = body.decls.iter().map(|d| (d.local, d.ty.clone())).collect();
        let mut blocks = body.blocks;
        assert_eq!(range_pass(&mut blocks, &locals, &HashSet::new()).map(|r| r.0), Ok(0));
    }
}
