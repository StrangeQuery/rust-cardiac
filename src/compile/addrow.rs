//! A loop that adds what doesn't change to a total a run-time number of times, `for _ in 0..d { t += x; }`, as hand
//! code does it: a computed jump into a row of ADDs (`Rvalue::Row`), one instruction an addition. The loop's count
//! must be known small (from its type, a mask, `% 10`, `digit()`, a promise or a test before the loop), since the
//! row holds an ADD for every addition it may make. Two totals or more are a row each.
//!
//! The loop, after the passes: a head that only tests a counter, and a body of one or more blocks, each the only way
//! into the next, back to the head, that only steps the counter by one, adds to totals (`t = t + x`, `t = t - x`,
//! `t = Lin(+1*t, ...)`, or an overflow-checked sum and its assert) and works out values that don't change from one
//! pass to the next (`x / 10`, `h + h`). It becomes one block: what doesn't change, worked out once; each total's
//! row (or, for a constant, n times the constant where that is smaller); the counter's last value, if it is read
//! after. When the count can't be below 0 the head's test goes too, and a count of 0 jumps to the row's end; else
//! it stays, and the row is entered for 1 or more.
//!
//! The sums agree with the loop's. The partial totals of one total only grow or only shrink, so a cell's three
//! digits of the last one are what the loop's stores kept at each step (the accumulator wraps at 10,000 the same
//! way); and an overflow check of the last one panics exactly when one of the loop's would have. A checked total
//! must stay within the accumulator's four digits for that: its start and its steps' sizes bound it.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use super::blocks::{self, Line};
use super::ir::{
    self, BasicBlock, BinOp, Block, Const, IntTy, Local, Operand, Place, Row, Rvalue, Sign, Statement, Terminator, Ty,
};
use super::ranges::{self, Range};
use super::util::Ordered;

/// The longest row: a count past this is left a loop.
pub const MOST: i64 = 60;

thread_local! {
    /// The rows not to make, by function and loop head: the program didn't fit with them (`super::compile_cut`).
    pub static SKIP: RefCell<HashSet<(String, BasicBlock)>> = RefCell::new(HashSet::new());
    /// The rows made, by function and loop head, with the cells their ADDs take: what to give up first.
    pub static MADE: RefCell<Vec<(String, BasicBlock, usize)>> = const { RefCell::new(Vec::new()) };
    /// No rows at all (a program in phases, or with a stack: see `super::compile_cut`).
    pub static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `f`, with rows made or not.
pub fn with_rows<T>(on: bool, f: impl FnOnce() -> T) -> T {
    let was = OFF.with(|o| o.replace(!on));
    let r = f();
    OFF.with(|o| o.set(was));
    r
}

/// What a line of the loop's body does.
enum Item {
    /// c = c + 1 (`up`) or c - 1
    Step(Local, bool),
    /// a total and what is added to it each pass
    Total(Total),
    /// t = rv, worked out the same every pass
    Fixed(Local, Rvalue),
}

struct Total {
    acc: Local,
    step: Step,
    sub: bool,
    check: Option<String>,
    /// the step read as the sum's first operand, `x + t`
    first: bool,
    /// how often its own lines name the total (its sum; for a checked one, the assert and the move out too)
    own: usize,
    /// a checked one's pair, `(t + x, overflowed)`
    pair: Option<Local>,
}

/// What a pass adds to a total: one operand, or a linear combination.
enum Step {
    One(Operand),
    Lin(Vec<(i64, Operand)>),
}

/// How a total's row is made: each pass's terms in the row, or a combination worked out once and its row.
enum Plan {
    Row(Vec<(i64, Operand)>),
    Once(Vec<(i64, Operand)>),
    /// a constant each pass, whose n times is on a card: added as n times the constant, with no row
    Times(i64),
}

/// Each loop of the shape, as a row; returns whether any was made.
pub fn add_rows(
    blocks: &mut Vec<Block>,
    locals: &mut Ordered<Local, Ty>,
    func: &str,
    promised: &HashSet<Local>,
    results: &HashSet<Local>,
    exact: &HashSet<Local>,
    early: Option<(&Edges, HashSet<Local>)>,
) -> bool {
    if OFF.with(|o| o.get()) {
        return false;
    }
    let mut tried: HashSet<BasicBlock> = HashSet::new();
    let mut any = false;
    loop {
        let types: HashMap<Local, Ty> = locals.iter().map(|(l, t)| (*l, t.clone())).collect();
        let Some(edges) = ranges::edge_ranges(blocks, &types, promised) else {
            break;
        };
        let huge = ranges::huge_usizes(blocks, &types, promised, None);
        let heads: Vec<BasicBlock> = blocks.iter().map(|b| b.name).filter(|h| !tried.contains(h)).collect();
        let mut made = false;
        for h in heads {
            tried.insert(h);
            let skipped = SKIP.with(|s| s.borrow().contains(&(func.to_string(), h)));
            if skipped {
                continue;
            }
            let cx = Context {
                func,
                types: &types,
                edges: &edges,
                early: early.as_ref().map(|(e, r)| (*e, r)),
                huge: &huge,
                results,
                exact,
            };
            if let Some(cells) = row(blocks, locals, h, &cx) {
                MADE.with(|m| m.borrow_mut().push((func.to_string(), h, cells)));
                made = true;
                any = true;
                break;
            }
        }
        if !made {
            break;
        }
    }
    if any {
        blocks::merge_blocks(blocks);
    }
    any
}

/// What holds on each way from one block to the next (`ranges::edge_ranges`).
pub type Edges = HashMap<(BasicBlock, BasicBlock), HashMap<Local, Range>>;

struct Context<'a> {
    /// the function: main runs once a run, any other may be called again and again
    func: &'a str,
    types: &'a HashMap<Local, Ty>,
    edges: &'a Edges,
    /// the same, from before the passes, while the promises (`assume`) still said what they hold; and the locals
    /// that hold other values since (the counters counted to 0)
    early: Option<(&'a Edges, &'a HashSet<Local>)>,
    huge: &'a HashSet<Local>,
    results: &'a HashSet<Local>,
    exact: &'a HashSet<Local>,
}

fn int(o: &Operand) -> Option<i128> {
    o.int().map(|(v, _)| v)
}

/// A whole local read by copy or move.
fn read(o: &Operand) -> Option<Local> {
    match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    }
}

/// An operand without how it is read: `move _3` is `copy _3`.
fn bare(o: &Operand) -> Operand {
    match o {
        Operand::Move(p) => Operand::Copy(p.clone()),
        o => o.clone(),
    }
}

fn copy(l: Local) -> Operand {
    Operand::Copy(Place::local(l))
}

fn card(value: i128) -> Operand {
    Operand::Constant(Const::Int { value, ty: IntTy::I16 })
}

/// A statement that does nothing at run time.
fn idle(s: &Statement) -> bool {
    matches!(
        s,
        Statement::Nop | Statement::StorageLive(_) | Statement::StorageDead(_)
    )
}

/// The loop whose head is block `h`, as a row: the cells its ADDs take, or None where it isn't one.
fn row(blocks: &mut Vec<Block>, locals: &mut Ordered<Local, Ty>, h: BasicBlock, cx: &Context) -> Option<usize> {
    let hi = ir::find(blocks, h)?;
    // (not the function's first block, which is entered from outside as well)
    if hi == 0 {
        return None;
    }
    let head = &blocks[hi];
    let stmts: Vec<&Statement> = head.statements.iter().filter(|s| !idle(s)).collect();
    let [Statement::Assign(bp, Rvalue::BinaryOp(cmp, left, right))] = stmts.as_slice() else {
        return None;
    };
    let b = bp.as_local()?;
    let Terminator::SwitchInt {
        discr,
        arms,
        otherwise: Some(other),
    } = &head.terminator
    else {
        return None;
    };
    let ([(0, zero)], Some(true)) = (arms.as_slice(), read(discr).map(|d| d == b)) else {
        return None;
    };
    if ir::occurrences(blocks, b) != 2 {
        return None;
    }
    let preds = ir::predecessors(blocks);
    // the body: from one of the ways out, blocks each entered only from the one before, back to the head
    let chain_from = |start: BasicBlock| -> Option<Vec<usize>> {
        let mut chain: Vec<usize> = vec![];
        let mut at = start;
        let mut from = h;
        loop {
            if at == h || chain.len() >= 6 {
                return None;
            }
            let i = ir::find(blocks, at)?;
            if preds.get(&at).map(|v| v.as_slice()) != Some(&[from][..]) || chain.contains(&i) {
                return None;
            }
            chain.push(i);
            let next = match &blocks[i].terminator {
                Terminator::Goto(n) => *n,
                t @ Terminator::Assert { target, .. } if ranges::overflow_assert(t).is_some() => *target,
                _ => return None,
            };
            if next == h {
                return Some(chain);
            }
            from = at;
            at = next;
        }
    };
    let (chain, exit, on_true) = match (chain_from(*other), chain_from(*zero)) {
        (Some(c), None) => (c, *zero, true),
        (None, Some(c)) => (c, *other, false),
        _ => return None,
    };
    // A row's cells cost a run the time to load them too, about three instructions each, which a loop main runs
    // once a run seldom wins back: only a loop that can run again in a run (inside another, or in a function other
    // than main) is made a row; one that runs once is made only n times a constant, where that takes fewer cells
    // than the loop.
    let again = {
        let mut seen: HashSet<BasicBlock> = HashSet::new();
        let mut todo = vec![exit];
        let mut found = false;
        while let Some(n) = todo.pop() {
            if n == h {
                found = true;
                break;
            }
            if seen.insert(n) {
                todo.extend(
                    ir::find(blocks, n)
                        .into_iter()
                        .flat_map(|i| blocks[i].terminator.successors()),
                );
            }
        }
        found
    };
    let once_a_run = cx.func == "main" && !again;
    let latch = blocks[*chain.last().unwrap()].name;
    let entries: Vec<BasicBlock> = preds.get(&h)?.iter().copied().filter(|p| *p != latch).collect();
    if entries.is_empty() || entries.iter().any(|p| chain.iter().any(|&i| blocks[i].name == *p)) {
        return None;
    }
    // the body's lines, in order
    let mut items: Vec<Item> = vec![];
    for (k, &i) in chain.iter().enumerate() {
        let blk = &blocks[i];
        let lines: Vec<&Statement> = blk.statements.iter().filter(|s| !idle(s)).collect();
        // a checked sum ends a block and its value is moved out first thing in the next
        let checked = ranges::overflow_assert(&blk.terminator).map(|(p, _)| p);
        let mut j = 0;
        if k > 0 {
            if let Some((p, _)) = ranges::overflow_assert(&blocks[chain[k - 1]].terminator) {
                // (taken with the sum, below)
                let Some(Statement::Assign(d, Rvalue::Use(Operand::Move(q) | Operand::Copy(q)))) = lines.first() else {
                    return None;
                };
                let moved =
                    q.base_local() == Some(p) && matches!(q.projection.as_slice(), [ir::Projection::Field(0, _)]);
                let Some(Item::Total(t)) = items.last() else {
                    return None;
                };
                if !moved || d.as_local() != Some(t.acc) {
                    return None;
                }
                j = 1;
            }
        }
        while j < lines.len() {
            let last = j + 1 == lines.len();
            let Statement::Assign(d, rv) = lines[j] else {
                return None;
            };
            let d = d.as_local()?;
            let item = match rv {
                // the overflow-checked sum, `p = AddWithOverflow(copy t, x)` or `p = SumWithOverflow(+copy t, ..)`,
                // its assert, and `t = move (p.0)`
                Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow), x, y)
                    if last && checked == Some(d) =>
                {
                    let sub = *op == BinOp::SubWithOverflow;
                    let acc = read(x)?;
                    let msg = assert_message(&blk.terminator)?;
                    Item::Total(Total {
                        acc,
                        step: Step::One(y.clone()),
                        sub,
                        check: Some(msg),
                        own: 2 + Line::Terminator(&blk.terminator)
                            .locals()
                            .iter()
                            .filter(|l| **l == acc)
                            .count(),
                        pair: Some(d),
                        first: false,
                    })
                }
                Rvalue::Sum(terms) if last && checked == Some(d) => {
                    let msg = assert_message(&blk.terminator)?;
                    let (acc, rest) = split_sum(terms)?;
                    Item::Total(Total {
                        acc,
                        step: rest,
                        sub: false,
                        check: Some(msg),
                        own: 2 + Line::Terminator(&blk.terminator)
                            .locals()
                            .iter()
                            .filter(|l| **l == acc)
                            .count(),
                        pair: Some(d),
                        first: false,
                    })
                }
                Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub), x, y) if read(x) == Some(d) || read(y) == Some(d) => {
                    let one = |o: &Operand| int(o) == Some(1);
                    if read(x) == Some(d) && one(y) {
                        Item::Step(d, *op == BinOp::Add)
                    } else if *op == BinOp::Add && read(y) == Some(d) && one(x) {
                        Item::Step(d, true)
                    } else if read(x) == Some(d) {
                        Item::Total(Total {
                            acc: d,
                            step: Step::One(y.clone()),
                            sub: *op == BinOp::Sub,
                            check: None,
                            own: 2,
                            pair: None,
                            first: false,
                        })
                    } else if *op == BinOp::Add {
                        Item::Total(Total {
                            acc: d,
                            step: Step::One(x.clone()),
                            sub: false,
                            check: None,
                            own: 2,
                            pair: None,
                            first: true,
                        })
                    } else {
                        return None;
                    }
                }
                Rvalue::BinaryOp(
                    BinOp::AddWrapping | BinOp::SubWrapping | BinOp::AddUnchecked | BinOp::SubUnchecked,
                    x,
                    y,
                ) if read(x) == Some(d) && int(y) == Some(1) => {
                    let up = matches!(rv, Rvalue::BinaryOp(BinOp::AddWrapping | BinOp::AddUnchecked, ..));
                    Item::Step(d, up)
                }
                Rvalue::Lin(terms) if terms.iter().any(|(k, o)| *k == 1 && read(o) == Some(d)) => {
                    let at = terms.iter().position(|(k, o)| *k == 1 && read(o) == Some(d))?;
                    let mut rest = terms.clone();
                    rest.remove(at);
                    Item::Total(Total {
                        acc: d,
                        step: Step::Lin(rest),
                        sub: false,
                        check: None,
                        own: 2,
                        pair: None,
                        first: false,
                    })
                }
                rv => Item::Fixed(d, rv.clone()),
            };
            items.push(item);
            j += 1;
        }
    }
    // a step by way of a local of its own, `_8 = Add(copy c, const 1); c = move _8`, is one step
    let mut k = 0;
    while k + 1 < items.len() {
        let stepped = match (&items[k], &items[k + 1]) {
            (Item::Fixed(t, Rvalue::BinaryOp(op, x, y)), Item::Fixed(c, Rvalue::Use(o))) => {
                let op = ranges::unwrapped(*op);
                let up = op == BinOp::Add;
                let by_one = matches!(op, BinOp::Add | BinOp::Sub | BinOp::AddUnchecked | BinOp::SubUnchecked)
                    && read(x) == Some(*c)
                    && int(y) == Some(1);
                let up = up || op == BinOp::AddUnchecked;
                (by_one && read(o) == Some(*t) && ir::occurrences(blocks, *t) == 2).then_some((*c, up))
            }
            _ => None,
        };
        if let Some((c, up)) = stepped {
            items.splice(k..k + 2, [Item::Step(c, up)]);
        }
        k += 1;
    }
    // what the loop writes: nothing read as fixed may be one of these, but a value worked out before it is read
    let mut written: HashSet<Local> = HashSet::from([b]);
    for &i in &chain {
        for s in &blocks[i].statements {
            if let Statement::Assign(d, _) = s {
                written.insert(d.base_local()?);
            }
        }
    }
    // the counter: the one step by one that the test reads (an overflow-checked one too, which the test keeps
    // within its bound); any other is a total that counts
    let tested = |l: Local| read(left) == Some(l) || read(right) == Some(l);
    let counters: Vec<(Local, bool)> = items
        .iter()
        .filter_map(|x| match x {
            Item::Step(c, up) if tested(*c) => Some((*c, *up)),
            Item::Total(t) if tested(t.acc) && t.check.is_some() => match &t.step {
                Step::One(o) if int(o) == Some(1) => Some((t.acc, !t.sub)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let [(c, up)] = counters.as_slice() else { return None };
    let (c, up) = (*c, *up);
    let items: Vec<Item> = items
        .into_iter()
        .map(|x| match x {
            Item::Total(t) if t.acc == c => Item::Step(c, up),
            Item::Step(d, up) if d != c => Item::Total(Total {
                acc: d,
                step: Step::One(card(1)),
                sub: !up,
                check: None,
                own: 2,
                pair: None,
                first: false,
            }),
            x => x,
        })
        .collect();
    // the test, as the condition the loop goes on while: lhs < rhs, lhs <= rhs or lhs != rhs
    let (op, lhs, rhs) = {
        let (op, l, r) = match cmp {
            BinOp::Gt => (BinOp::Lt, right, left),
            BinOp::Ge => (BinOp::Le, right, left),
            op => (*op, left, right),
        };
        match (on_true, op) {
            (true, BinOp::Lt | BinOp::Le | BinOp::Ne) => (op, l, r),
            (false, BinOp::Lt) => (BinOp::Le, r, l),
            (false, BinOp::Le) => (BinOp::Lt, r, l),
            (false, BinOp::Eq) => (BinOp::Ne, l, r),
            _ => return None,
        }
    };
    let bias = i128::from(op == BinOp::Le);
    // the count: e - c going up (c < e), c - e going down (e < c)
    let (e, c_left) = match (read(lhs) == Some(c), read(rhs) == Some(c)) {
        (true, false) => (rhs, true),
        (false, true) => (lhs, false),
        _ => return None,
    };
    if op != BinOp::Ne && up != c_left {
        return None;
    }
    let fixed_local = |l: Local| !written.contains(&l) && !cx.huge.contains(&l);
    let invariant = |o: &Operand| int(o).is_some() || read(o).is_some_and(fixed_local);
    if !invariant(e) || cx.huge.contains(&c) {
        return None;
    }
    // what holds on the way in; for the count's operands, what held there before the passes too
    let range_in = |o: &Operand, counted: bool| -> Option<Range> {
        if let Some(v) = int(o) {
            return Some([v as i64, v as i64]);
        }
        let l = read(o)?;
        let whole = ranges::type_range(cx.types.get(&l)?);
        let mut hull: Option<Range> = None;
        for p in &entries {
            let r = cx.edges.get(&(*p, h)).and_then(|f| f.get(&l).copied()).or(whole)?;
            hull = Some(hull.map_or(r, |q| [q[0].min(r[0]), q[1].max(r[1])]));
        }
        // and what held at the head before the passes, on any way in (the loop's own too): both hold on the way in
        let early = cx
            .early
            .filter(|(_, changed)| counted && !changed.contains(&l))
            .and_then(|(edges, _)| {
                let ins: Vec<Option<Range>> = edges
                    .iter()
                    .filter(|((_, to), _)| *to == h)
                    .map(|(_, f)| f.get(&l).copied().or(whole))
                    .collect();
                if ins.is_empty() || ins.iter().any(|r| r.is_none()) {
                    return None;
                }
                let ins: Vec<Range> = ins.into_iter().flatten().collect();
                Some([ins.iter().map(|r| r[0]).min()?, ins.iter().map(|r| r[1]).max()?])
            });
        match (hull, early) {
            (Some(a), Some(b)) if a[0].max(b[0]) <= a[1].min(b[1]) => Some([a[0].max(b[0]), a[1].min(b[1])]),
            (hull, _) => hull,
        }
    };
    let entry_range = |o: &Operand| range_in(o, false);
    let (rc, re) = (range_in(&copy(c), true)?, range_in(e, true)?);
    let count = if up {
        [re[0] - rc[1], re[1] - rc[0]]
    } else {
        [rc[0] - re[1], rc[1] - re[0]]
    };
    let count = [count[0] + bias as i64, count[1] + bias as i64];
    let most = count[1];
    if !(1..=MOST).contains(&most) || count[0] == count[1] || (op == BinOp::Ne && count[0] < 0) {
        return None;
    }
    let guarded = count[0] < 0;
    // the values worked out once, and the totals
    let mut fixed: HashMap<Local, Range> = HashMap::new();
    // an element of an array the loop doesn't write, at an index it doesn't change (`v[c]`, `v[2]`): its type's
    // range
    let element = |o: &Operand| -> Option<Range> {
        let (Operand::Copy(p) | Operand::Move(p)) = o else {
            return None;
        };
        let base = p.base_local().filter(|b| fixed_local(*b))?;
        let fixed_index = match p.projection.as_slice() {
            [ir::Projection::Index(i) | ir::Projection::IndexPlus(i, _)] => fixed_local(*i),
            [ir::Projection::ConstantIndex { .. }] => true,
            _ => false,
        };
        match cx.types.get(&base) {
            Some(Ty::Array(e, _)) if fixed_index => ranges::type_range(e),
            _ => None,
        }
    };
    let ok = |o: &Operand, fixed: &HashMap<Local, Range>| {
        int(o).is_some() || read(o).is_some_and(|l| fixed_local(l) || fixed.contains_key(&l)) || element(o).is_some()
    };
    // an operand's range: what its cell keeps, for a value worked out once
    let range_of = |o: &Operand, fixed: &HashMap<Local, Range>| -> Option<Range> {
        match read(o).and_then(|l| fixed.get(&l)) {
            Some(r) if r[0] >= -999 && r[1] <= 999 => Some(*r),
            Some(_) => Some([-999, 999]),
            None => element(o).or_else(|| entry_range(o)),
        }
    };
    let on_card = |r: Range| r[0] >= -999 && r[1] <= 999;
    // the lines of the row's block: what is worked out once, then each total's sum
    let mut once: Vec<Statement> = vec![];
    let mut sums: Vec<(Local, Plan, Option<String>)> = vec![];
    for x in &items {
        match x {
            Item::Step(..) => {}
            Item::Fixed(t, rv) => {
                let r = pure_range(rv, &|o| if ok(o, &fixed) { range_of(o, &fixed) } else { None })?;
                let i16 = matches!(cx.types.get(t), Some(Ty::Int(IntTy::I16 | IntTy::Isize)));
                if !i16 || fixed.contains_key(t) || cx.results.contains(t) || cx.exact.contains(t) {
                    return None;
                }
                // named in the loop alone, and set before it is read
                let inside: usize = chain
                    .iter()
                    .map(|&i| blocks[i].lines().map(|l| l.count(*t)).sum::<usize>())
                    .sum();
                if inside != ir::occurrences(blocks, *t) {
                    return None;
                }
                // a value read whole from the accumulator by the line after it would be read from its cell once
                // the lines between are gone: so a value read as the first operand of another must be on a card
                let first_read = rv
                    .operands()
                    .first()
                    .and_then(|o| read(o))
                    .and_then(|l| fixed.get(&l).copied());
                if matches!(rv, Rvalue::BinaryOp(..)) && first_read.is_some_and(|r| !on_card(r)) {
                    return None;
                }
                fixed.insert(*t, r);
                once.push(Statement::Assign(Place::local(*t), rv.clone()));
            }
            Item::Total(total) => {
                let acc = total.acc;
                let i16 = matches!(cx.types.get(&acc), Some(Ty::Int(IntTy::I16 | IntTy::Isize)));
                if !i16 || acc == c || acc == b || cx.huge.contains(&acc) || cx.exact.contains(&acc) {
                    return None;
                }
                if sums.iter().any(|(t, ..)| *t == acc) || read(e) == Some(acc) {
                    return None;
                }
                // named in the loop by its own lines alone (and a checked one's pair too)
                let inside: usize = chain
                    .iter()
                    .map(|&i| blocks[i].lines().map(|l| l.count(acc)).sum::<usize>())
                    .sum();
                if inside != total.own {
                    return None;
                }
                if let Some(p) = total.pair {
                    let named: usize = chain
                        .iter()
                        .map(|&i| blocks[i].lines().map(|l| l.count(p)).sum::<usize>())
                        .sum();
                    if named != ir::occurrences(blocks, p) {
                        return None;
                    }
                }
                let terms: Vec<(i64, Operand)> = match &total.step {
                    Step::One(o) => vec![(if total.sub { -1 } else { 1 }, o.clone())],
                    // (a term named twice is one, twice over: `x + x` is 2 x)
                    Step::Lin(terms) => {
                        let mut merged: Vec<(i64, Operand)> = vec![];
                        for (k, o) in terms {
                            match merged.iter_mut().find(|(_, m)| bare(m) == bare(o)) {
                                Some((j, _)) => *j += k,
                                None => merged.push((*k, o.clone())),
                            }
                        }
                        merged.retain(|(k, _)| *k != 0);
                        merged
                    }
                };
                if terms.is_empty() || !terms.iter().all(|(k, o)| *k != 0 && ok(o, &fixed) && int(o) != Some(0)) {
                    return None;
                }
                // a value worked out once and read whole as the sum's first operand, past a card, would be read
                // from its cell
                if let (Step::One(o), true) = (&total.step, total.first) {
                    if read(o).and_then(|l| fixed.get(&l)).is_some_and(|r| !on_card(*r)) {
                        return None;
                    }
                }
                let ra = entry_range(&copy(acc))?;
                let size = |terms: &[(i64, Operand)]| -> Option<i64> {
                    let each = terms.iter().try_fold(0i64, |s, (k, o)| {
                        let r = range_of(o, &fixed)?;
                        Some(s + k.abs() * r[0].abs().max(r[1].abs()))
                    })?;
                    Some(ra[0].abs().max(ra[1].abs()) + most * each)
                };
                let length: i64 = terms.iter().map(|(k, _)| k.abs()).sum();
                // a constant: n times it, where that is on a card and so is the total for a checked one
                let constant = match terms.as_slice() {
                    [(k, o)] => int(o).map(|v| *k * v as i64),
                    _ => None,
                };
                let total_on_card = |v: i64| {
                    let r = [ra[0] + (v * most).min(0), ra[1] + (v * most).max(0)];
                    total.check.is_none() || on_card(r)
                };
                // (where n times the constant takes more cells than the row would, the row)
                let cheap = |v: i64| v.abs() == 1 || digits(v) + 2 <= most + 6;
                let plan = if let Some(v) = constant.filter(|v| v.abs() * most <= 999 && total_on_card(*v) && cheap(*v))
                {
                    Plan::Times(v)
                } else if length == 1 || terms.len() == 1 {
                    // one operand (k times a pass): the partial totals only grow or only shrink, so a check of the
                    // last is a check of each; the accumulator's four digits must hold them for that
                    if total.check.is_some() && size(&terms)? > 9999 {
                        return None;
                    }
                    Plan::Row(terms)
                } else if lin_range(&terms, &|o| range_of(o, &fixed)).is_some_and(on_card) {
                    // a combination on a card: worked out once, into a local of its own
                    Plan::Once(terms)
                } else {
                    // else each pass's terms in the row, which needs the four digits for every partial total
                    if size(&terms)? > 9999 {
                        return None;
                    }
                    Plan::Row(terms)
                };
                sums.push((acc, plan, total.check.clone()));
            }
        }
    }
    if sums.is_empty() {
        return None;
    }
    let length = |plan: &Plan| -> i64 {
        match plan {
            Plan::Row(terms) => terms.iter().map(|(k, _)| k.abs()).sum(),
            Plan::Once(_) => 1,
            Plan::Times(_) => 0,
        }
    };
    if sums.iter().any(|(_, p, _)| length(p) * most > MOST) {
        return None;
    }
    if once_a_run
        && !sums
            .iter()
            .all(|(_, p, _)| matches!(p, Plan::Times(v) if digits(*v) <= 3))
    {
        return None;
    }
    // the counter after the loop: e (or e + 1), when anything reads it
    let live = blocks::live_after_returning(blocks, cx.results);
    let ei = ir::find(blocks, exit)?;
    let reads_after = live
        .get(ei)
        .and_then(|l| l.first())
        .is_some_and(|l| l.contains(&c) || blocks[ei].each_line().next().is_some_and(|x| x.locals().contains(&c)));
    let c_after = |e: &Operand| -> Option<Statement> {
        if bias == 0 {
            Some(Statement::Assign(Place::local(c), Rvalue::Use(e.clone())))
        } else {
            int(e).map(|k| Statement::Assign(Place::local(c), Rvalue::Use(card(k + 1))))
        }
    };
    let last_c = if reads_after { Some(c_after(e)?) } else { None };
    // the count, as an operand
    let mut out: Vec<Statement> = vec![];
    let new_local = |locals: &mut Ordered<Local, Ty>| -> Local {
        let n = locals
            .keys()
            .map(|l| l.0)
            .filter(|&n| n != Local::UNIT.0)
            .max()
            .unwrap_or(0)
            + 1;
        let l = Local(n);
        locals.set(l, Ty::Int(IntTy::I16));
        l
    };
    let times: Operand = if up {
        // e - c (+ 1)
        match (int(e), rc[0] == rc[1]) {
            (_, true) if rc[0] as i128 - bias == 0 && read(e).is_some() => copy(read(e).unwrap()),
            (Some(k), _) => {
                let n = new_local(locals);
                out.push(Statement::Assign(
                    Place::local(n),
                    Rvalue::BinaryOp(BinOp::Sub, card(k + bias), copy(c)),
                ));
                copy(n)
            }
            (None, true) => {
                let n = new_local(locals);
                let k = rc[0] as i128 - bias;
                out.push(Statement::Assign(
                    Place::local(n),
                    Rvalue::BinaryOp(BinOp::Sub, copy(read(e)?), card(k)),
                ));
                copy(n)
            }
            (None, false) if bias == 0 => {
                let n = new_local(locals);
                out.push(Statement::Assign(
                    Place::local(n),
                    Rvalue::BinaryOp(BinOp::Sub, copy(read(e)?), copy(c)),
                ));
                copy(n)
            }
            _ => return None,
        }
    } else {
        // c - e (+ 1)
        match int(e) {
            Some(k) if k - bias == 0 => copy(c),
            Some(k) => {
                let n = new_local(locals);
                out.push(Statement::Assign(
                    Place::local(n),
                    Rvalue::BinaryOp(BinOp::Sub, copy(c), card(k - bias)),
                ));
                copy(n)
            }
            None if bias == 0 => {
                let n = new_local(locals);
                out.push(Statement::Assign(
                    Place::local(n),
                    Rvalue::BinaryOp(BinOp::Sub, copy(c), copy(read(e)?)),
                ));
                copy(n)
            }
            None => return None,
        }
    };
    let as_copy = |o: &Operand| match o {
        Operand::Move(p) => Operand::Copy(p.clone()),
        o => o.clone(),
    };
    for mut st in once {
        if let Statement::Assign(_, rv) = &mut st {
            for o in rv.operands_mut() {
                *o = as_copy(o);
            }
        }
        out.push(st);
    }
    let mut cells = 0;
    for (acc, plan, check) in sums {
        let terms = match plan {
            Plan::Times(v) => {
                let by = if v.abs() == 1 {
                    times.clone()
                } else {
                    let p = new_local(locals);
                    let product = Rvalue::BinaryOp(BinOp::Mul, times.clone(), card(v.abs() as i128));
                    out.push(Statement::Assign(Place::local(p), product));
                    copy(p)
                };
                let op = if v > 0 { BinOp::Add } else { BinOp::Sub };
                out.push(Statement::Assign(
                    Place::local(acc),
                    Rvalue::BinaryOp(op, copy(acc), by),
                ));
                continue;
            }
            Plan::Row(terms) => terms,
            Plan::Once(terms) => {
                let s = new_local(locals);
                let terms = terms.iter().map(|(k, o)| (*k, as_copy(o))).collect();
                out.push(Statement::Assign(Place::local(s), Rvalue::Lin(terms)));
                vec![(1, copy(s))]
            }
        };
        // each pass's ADDs and SUBs, a term k times over as k of them
        let mut steps: Vec<(Sign, Operand)> = vec![];
        for (k, o) in &terms {
            let sign = if *k > 0 { Sign::Plus } else { Sign::Minus };
            for _ in 0..k.abs() {
                steps.push((sign, as_copy(o)));
            }
        }
        cells += most as usize * steps.len();
        out.push(Statement::Assign(
            Place::local(acc),
            Rvalue::Row(Box::new(Row {
                acc: copy(acc),
                steps,
                times: times.clone(),
                most: most as u32,
                check,
            })),
        ));
    }
    out.extend(last_c);
    // the body is one block now, which goes on to the loop's exit; the head's test goes where the count can't be
    // below 0
    let first = blocks[chain[0]].name;
    let gone: HashSet<BasicBlock> = chain[1..].iter().map(|&i| blocks[i].name).collect();
    let bi = chain[0];
    blocks[bi].statements = out;
    blocks[bi].terminator = Terminator::Goto(exit);
    if !guarded {
        let hi = ir::find(blocks, h)?;
        blocks[hi]
            .statements
            .retain(|s| !matches!(s, Statement::Assign(d, _) if d.as_local() == Some(b)));
        blocks[hi].terminator = Terminator::Goto(first);
    }
    blocks.retain(|x| !gone.contains(&x.name));
    Some(cells)
}

/// The cells n times a constant takes: an add for each unit of its digits, and a shift for each digit after the
/// first.
fn digits(v: i64) -> i64 {
    let s = v.abs().to_string();
    s.bytes().map(|b| (b - b'0') as i64).sum::<i64>() + s.len() as i64 - 1
}

/// The message of an overflow check's assert.
fn assert_message(t: &Terminator) -> Option<String> {
    match t {
        Terminator::Assert { msg, .. } => Some(msg.clone()),
        _ => None,
    }
}

/// `SumWithOverflow(+copy t, ..)`: t, and the rest as what a pass adds.
fn split_sum(terms: &[(Sign, Operand)]) -> Option<(Local, Step)> {
    let at = terms.iter().position(|(s, o)| *s == Sign::Plus && read(o).is_some())?;
    let acc = read(&terms[at].1)?;
    let rest: Vec<(i64, Operand)> = terms
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != at)
        .map(|(_, (s, o))| (if *s == Sign::Plus { 1 } else { -1 }, o.clone()))
        .collect();
    Some((acc, Step::Lin(rest)))
}

/// The range of a linear combination, from its terms' ranges.
fn lin_range(terms: &[(i64, Operand)], range: &dyn Fn(&Operand) -> Option<Range>) -> Option<Range> {
    terms.iter().try_fold([0i64, 0i64], |sum, (k, o)| {
        let r = range(o)?;
        let (a, b) = (r[0].saturating_mul(*k), r[1].saturating_mul(*k));
        Some([sum[0].saturating_add(a.min(b)), sum[1].saturating_add(a.max(b))])
    })
}

/// The range of a value worked out without a check (none that can panic), from its operands' ranges; None for
/// anything else.
fn pure_range(rv: &Rvalue, range: &dyn Fn(&Operand) -> Option<Range>) -> Option<Range> {
    match rv {
        Rvalue::Use(o) => range(o),
        Rvalue::Lin(terms) => lin_range(terms, range),
        Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub), a, b) => {
            let terms = [(1, a.clone()), (if *op == BinOp::Add { 1 } else { -1 }, b.clone())];
            lin_range(&terms, range)
        }
        Rvalue::BinaryOp(BinOp::Mul, a, b) if int(b).is_some() => lin_range(&[(int(b)? as i64, a.clone())], range),
        Rvalue::BinaryOp(op @ (BinOp::Div | BinOp::Rem | BinOp::UDiv | BinOp::URem), a, b) => {
            let k = int(b)? as i64;
            let r = range(a)?;
            if k <= 0 || (matches!(op, BinOp::UDiv | BinOp::URem) && r[0] < 0) {
                return None;
            }
            Some(match op {
                BinOp::Div | BinOp::UDiv => [r[0] / k, r[1] / k],
                _ => [if r[0] < 0 { -(k - 1) } else { 0 }, if r[1] > 0 { k - 1 } else { 0 }],
            })
        }
        Rvalue::UnaryOp(ir::UnOp::Neg, a) => range(a).map(|r| [-r[1], -r[0]]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The blocks of a body, rowed, each line as it prints.
    fn rowed(text: &str) -> Vec<String> {
        let body = ir::parse_body(text);
        let mut locals: Ordered<Local, Ty> = Ordered::new();
        for d in &body.decls {
            locals.set(d.local, d.ty.clone());
        }
        let mut blocks = body.blocks;
        let none = HashSet::new();
        add_rows(&mut blocks, &mut locals, "f", &none, &none, &none, None);
        blocks
            .iter()
            .flat_map(|b| {
                std::iter::once(format!("{}:", b.name))
                    .chain(b.statements.iter().map(|s| s.to_string()))
                    .chain([b.terminator.to_string()])
            })
            .collect()
    }

    /// `t = x; for _ in 0..(d & COUNT) { t += x }; return t`, in the IR the passes leave.
    fn masked(count: i16, step: &str) -> String {
        format!(
            "let mut _0: i16;\n let _1: i16;\n let _2: i16;\n let mut _3: i16;\n let mut _4: bool;\n bb0: {{\n \
             _3 = BitAnd(copy _1, const {count}_i16);\n _0 = copy _2;\n goto -> bb1;\n }}\n bb1: {{\n \
             _4 = Lt(const 0_i16, copy _3);\n switchInt(move _4) -> [0: bb3, otherwise: bb2];\n }}\n bb2: {{\n \
             _3 = Sub(copy _3, const 1_i16);\n _0 = Add(copy _0, {step});\n goto -> bb1;\n }}\n bb3: {{\n \
             return;\n }}\n"
        )
    }

    #[test]
    fn a_count_known_not_below_0_is_a_row_with_no_test() {
        assert_eq!(
            rowed(&masked(7, "copy _2")),
            [
                "bb0:",
                "_3 = BitAnd(copy _1, const 7_i16);",
                "_0 = copy _2;",
                "_0 = Row(copy _0, [+copy _2], copy _3, 7);",
                "return;",
            ]
        );
    }

    #[test]
    fn a_count_that_may_be_below_0_keeps_the_test() {
        let text = masked(7, "copy _2").replace("BitAnd(copy _1, const 7_i16)", "Rem(copy _1, const 10_i16)");
        assert_eq!(
            rowed(&text),
            [
                "bb0:",
                "_3 = Rem(copy _1, const 10_i16);",
                "_0 = copy _2;",
                "_4 = Lt(const 0_i16, copy _3);",
                "switchInt(move _4) -> [0: bb3, otherwise: bb2];",
                "bb2:",
                "_0 = Row(copy _0, [+copy _2], copy _3, 9);",
                "goto -> bb3;",
                "bb3:",
                "return;",
            ]
        );
    }

    #[test]
    fn a_constant_is_added_as_the_count_times_it() {
        let out = rowed(&masked(7, "const 1_i16"));
        assert!(out.contains(&"_0 = Add(copy _0, copy _3);".to_string()), "{out:?}");
    }

    #[test]
    fn a_count_that_can_be_large_stays_a_loop() {
        let text = masked(127, "copy _2");
        let before: Vec<String> = ir::parse_body(&text)
            .blocks
            .iter()
            .flat_map(|b| {
                std::iter::once(format!("{}:", b.name))
                    .chain(b.statements.iter().map(|s| s.to_string()))
                    .chain([b.terminator.to_string()])
            })
            .collect();
        assert_eq!(rowed(&text), before);
    }
}
