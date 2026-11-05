//! Passes that remove copies: copy forwarding, element and field forwarding, copies at a join, and sharing cells.
use std::collections::{HashMap, HashSet};

use super::blocks::{addressed_locals, copy_unless_last, live_after, live_after_returning, Line};
use super::ir::{
    blocks_naming, find, lines_naming, occurrences, BasicBlock, BinOp, Block, CastKind, Const, IntTy, Local, Operand,
    Path, Place, PlaceBase, Projection, Rvalue, Segment, Statement, Terminator, Ty, UnOp,
};
use super::lower::{Sig, Sym};
use super::util::Ordered;

/// The block a terminator goes on to: a call's return, a passed assert, or a goto.
fn next_in_run(t: &Terminator) -> Option<BasicBlock> {
    match t {
        Terminator::Call { target, .. } => *target,
        Terminator::Assert { target, .. } => Some(*target),
        Terminator::Goto(b) => Some(*b),
        _ => None,
    }
}

/// The block a call returns to, or a goto goes to.
fn return_or_goto(t: &Terminator) -> Option<BasicBlock> {
    match t {
        Terminator::Call { target, .. } => *target,
        Terminator::Goto(b) => Some(*b),
        _ => None,
    }
}

/// A type a single cell holds whole, or a reference or pointer (an address).
fn cell_type(t: Option<&Ty>) -> bool {
    matches!(
        t,
        Some(Ty::Int(IntTy::U8 | IntTy::I16 | IntTy::Usize) | Ty::Bool | Ty::Ref { .. } | Ty::Ptr { .. })
    )
}

/// u8, i16, bool or usize.
fn scalar(types: &HashMap<Local, Ty>, x: Local) -> bool {
    matches!(
        types.get(&x),
        Some(Ty::Int(IntTy::U8 | IntTy::I16 | IntTy::Usize) | Ty::Bool)
    )
}

/// Does the line assign `n`, whole (`n = ...`) or a field of it and what is indexed off that (`(n.k: T) = ...`)?
fn assigns(line: Line, n: &PlaceBase) -> bool {
    let by = |p: &Place| {
        p.base == *n
            && (p.projection.is_empty() || (matches!(p.projection[0], Projection::Field(..)) && p.wrapped() == 1))
    };
    match line {
        Line::Statement(Statement::Assign(p, _)) => by(p),
        Line::Terminator(Terminator::Call { dest, .. }) => by(dest),
        _ => false,
    }
}

/// Is the address of `n` taken anywhere, of the local itself or at an index: `&n`, `&mut n[i]`?
fn address_taken(blocks: &[Block], n: &PlaceBase) -> bool {
    blocks
        .iter()
        .flat_map(|b| &b.statements)
        .any(|s| matches!(s, Statement::Assign(_, Rvalue::Ref(_, p)) if p.base == *n && p.wrapped() == 0))
}

/// Every place in a line, mutably.
fn places_mut(b: &mut Block, i: usize, f: &mut dyn FnMut(&mut Place)) {
    if i < b.statements.len() {
        b.statements[i].places_mut(f);
    } else {
        b.terminator.places_mut(f);
    }
}

/// Name `to` wherever a line names local `from`: as a place's local, or as an index.
fn rename(b: &mut Block, from: Local, to: &PlaceBase) {
    let to_local = match to {
        PlaceBase::Local(l) => Some(*l),
        PlaceBase::Result { .. } => None,
    };
    let f = &mut |p: &mut Place| {
        if p.base == PlaceBase::Local(from) {
            p.base = to.clone();
        }
        if let Some(t) = to_local {
            for x in p.projection.iter_mut() {
                if let Projection::Index(i) | Projection::IndexPlus(i, _) = x {
                    if *i == from {
                        *i = t;
                    }
                }
            }
        }
    };
    for i in 0..=b.statements.len() {
        places_mut(b, i, f);
    }
}

/// Is `x` an index anywhere in the block?
fn indexes(b: &Block, x: Local) -> bool {
    let places: Vec<&Place> = b
        .statements
        .iter()
        .flat_map(|s| s.places())
        .chain(b.terminator.places())
        .collect();
    places.iter().any(|p| {
        p.projection
            .iter()
            .any(|q| matches!(q, Projection::Index(i) | Projection::IndexPlus(i, _) if *i == x))
    })
}

/// Copy forwarding. `_k = copy _n` where _k is a compiler temporary used only in its own block B and in the blocks
/// straight after it (each reached from nowhere else): if _n cannot change in between and its address is never
/// taken, every use of _k can read _n directly and the copy disappears.
pub fn forward_copies(
    blocks: &mut [Block],
    types: &HashMap<Local, Ty>,
    named: &HashSet<Local>,
    params: &HashSet<Local>,
) {
    let mut forwarded = false;
    let mut changed = true;
    while changed {
        changed = false;
        'blocks: for bi in 0..blocks.len() {
            let mut i = 0;
            while i < blocks[bi].statements.len() {
                if forward_element(blocks, bi, i, types, named, params) {
                    changed = true;
                    break 'blocks;
                }
                // `_k = copy _n`, or a u8 widened to usize, which keeps its value, so it forwards like a copy (an i16
                // is checked first)
                let Statement::Assign(d, rv) = &blocks[bi].statements[i] else {
                    i += 1;
                    continue;
                };
                let (src, cast) = match rv {
                    Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => (p, false),
                    Rvalue::Cast(Operand::Copy(p) | Operand::Move(p), Ty::Int(IntTy::Usize), kind)
                        if *kind == CastKind::IntToInt =>
                    {
                        (p, true)
                    }
                    _ => {
                        i += 1;
                        continue;
                    }
                };
                let (Some(k), true) = (d.as_local().filter(|l| *l != Local::UNIT), src.projection.is_empty()) else {
                    i += 1;
                    continue;
                };
                let n = src.base.clone();
                if let PlaceBase::Local(l) = n {
                    if l == Local::UNIT {
                        i += 1;
                        continue;
                    }
                }
                let from = match n {
                    PlaceBase::Local(l) => types.get(&l),
                    PlaceBase::Result { .. } => None,
                };
                if cast && from != Some(&Ty::Int(IntTy::U8)) && from != Some(&Ty::Int(IntTy::Usize)) {
                    i += 1;
                    continue;
                }
                // a cell of a function's result (@f.N) holds until the next call
                let result = matches!(n, PlaceBase::Result { .. });
                if k == Local(0) || named.contains(&k) || params.contains(&k) || !cell_type(types.get(&k)) {
                    i += 1;
                    continue;
                }
                if address_taken(blocks, &n) {
                    i += 1;
                    continue;
                }
                let defs: usize = blocks
                    .iter()
                    .map(|b| b.defs().iter().filter(|d| **d == Some(k)).count())
                    .sum();
                if defs != 1 {
                    i += 1;
                    continue;
                }
                // the straight run of blocks after B: each the only way into the next
                let mut run: Vec<usize> = vec![];
                let mut u = bi;
                while let Some(s) = next_in_run(&blocks[u].terminator).and_then(|s| find(blocks, s)) {
                    if s == bi || run.contains(&s) || blocks_naming(blocks, blocks[s].name) != 1 {
                        break;
                    }
                    run.push(s);
                    u = s;
                }
                let users: Vec<usize> = (0..blocks.len())
                    .filter(|&x| x != bi && blocks[x].lines().any(|l| l.mentions(k)))
                    .collect();
                if !users.iter().all(|x| run.contains(x)) {
                    i += 1;
                    continue;
                }
                let reach: i64 = users
                    .iter()
                    .map(|x| run.iter().position(|r| r == x).unwrap() as i64)
                    .max()
                    .unwrap_or(-1);
                // a result cell: no call before the last read
                if result && reach >= 0 {
                    let upto = &run[..(reach.max(0) as usize).min(run.len())];
                    if std::iter::once(&bi)
                        .chain(upto)
                        .any(|&x| !matches!(blocks[x].terminator, Terminator::Goto(_)))
                    {
                        i += 1;
                        continue;
                    }
                }
                // n must not change while k is still read (in B: up to k's last read; a statement that reads k and
                // sets n reads first, so it is fine)
                let lines: Vec<Line> = blocks[bi].each_line().collect();
                let last_b = (0..lines.len())
                    .rfind(|&j| j > i && lines[j].locals().contains(&k))
                    .unwrap_or(i);
                let end = if users.is_empty() { last_b } else { lines.len() };
                let later = if end > i + 1 { &lines[i + 1..end] } else { &[][..] };
                let changes_n = later.iter().any(|l| assigns(*l, &n))
                    || run[..((reach + 1) as usize).min(run.len())]
                        .iter()
                        .any(|&x| blocks[x].each_line().any(|l| assigns(l, &n)));
                if !result && changes_n {
                    i += 1;
                    continue;
                }
                // a result cell can't be an index
                if result
                    && std::iter::once(bi)
                        .chain(users.iter().copied())
                        .any(|x| indexes(&blocks[x], k))
                {
                    i += 1;
                    continue;
                }
                blocks[bi].statements.remove(i);
                for x in std::iter::once(bi).chain(users) {
                    rename(&mut blocks[x], k, &n);
                }
                changed = true;
                forwarded = true;
                break 'blocks;
            }
        }
    }
    // `_k = copy _n; switchInt(move _k)` is now `switchInt(move _n)`: that is _n's last read only if nothing reads
    // _n later
    if forwarded {
        copy_unless_last(blocks);
    }
}

/// A constant printed as one word: `5_i16`, `-3_i16`, `true`.
fn one_word_const(c: &Const) -> bool {
    match c {
        Const::Int { .. } | Const::Bool(_) => true,
        Const::Path(p) => p.qself.is_none() && matches!(p.segments.as_slice(), [Segment::Name { args: None, .. }]),
        _ => false,
    }
}

/// `_k = copy xs[i]` read once, later on (in this block or straight after it): the one read takes the element
/// itself, if neither xs nor i changes in between. And a constant only stored somewhere later is stored there
/// directly.
fn forward_element(
    blocks: &mut [Block],
    bi: usize,
    i: usize,
    types: &HashMap<Local, Ty>,
    named: &HashSet<Local>,
    params: &HashSet<Local>,
) -> bool {
    // `_k = const c` that is only stored somewhere later in the block (a struct literal put into an array, say)
    if let Statement::Assign(d, Rvalue::Use(Operand::Constant(c))) = &blocks[bi].statements[i] {
        if let (Some(k), true) = (d.as_local().filter(|l| *l != Local::UNIT), one_word_const(c)) {
            if !named.contains(&k) && !params.contains(&k) {
                let c = c.clone();
                let b = &blocks[bi];
                let lines: Vec<Line> = b.each_line().collect();
                let j = (i + 1..lines.len()).find(|&j| lines[j].locals().contains(&k));
                if let (Some(j), 2) = (j, lines_naming(blocks, k)) {
                    let moved = |o: &Operand| matches!(o, Operand::Move(p) if p.as_local() == Some(k));
                    let target = blocks[bi].statements.get(j).cloned();
                    let rewritten = match target {
                        Some(Statement::Assign(dest, Rvalue::Use(o))) if moved(&o) => {
                            Some(Statement::Assign(dest, Rvalue::Use(Operand::Constant(c))))
                        }
                        Some(Statement::Assign(dest, Rvalue::Repeat(o, n))) if moved(&o) => {
                            Some(Statement::Assign(dest, Rvalue::Repeat(Operand::Constant(c), n)))
                        }
                        _ => None,
                    };
                    if let Some(s) = rewritten {
                        blocks[bi].statements[j] = s;
                        blocks[bi].statements.remove(i);
                        return true;
                    }
                }
            }
        }
    }
    let Statement::Assign(d, Rvalue::Use(Operand::Copy(el))) = &blocks[bi].statements[i] else {
        return false;
    };
    let Some(k) = d.as_local().filter(|l| *l != Local::UNIT) else {
        return false;
    };
    let (Some(a), [index]) = (el.base_local(), el.projection.as_slice()) else {
        return false;
    };
    let idx = match index {
        Projection::Index(x) | Projection::IndexPlus(x, _) => Some(*x),
        Projection::ConstantIndex { .. } => None,
        _ => return false,
    };
    if named.contains(&k) || params.contains(&k) || !scalar(types, k) {
        return false;
    }
    if occurrences(blocks, k) != 2 {
        return false;
    }
    let el = el.clone();
    // a line that writes xs (an element, or the whole of it) or sets the index
    let writes = |l: Line| -> bool {
        let dest = match l {
            Line::Statement(Statement::Assign(p, _)) => Some(p),
            Line::Terminator(Terminator::Call { dest, .. }) => Some(dest),
            _ => None,
        };
        let Some(p) = dest else { return false };
        let element = p.base == PlaceBase::Local(a) && p.wrapped() == 0 && !p.projection.is_empty();
        element || p.as_local() == Some(a) || (idx.is_some() && p.as_local() == idx)
    };
    // the read: later in this block, or further down a straight run of blocks (each reached only from the one before)
    let (mut u, mut j) = (bi, i + 1);
    loop {
        let lines: Vec<Line> = blocks[u].each_line().collect();
        while j < lines.len() && lines[j].count_of(k) == 0 {
            if writes(lines[j]) {
                return false;
            }
            j += 1;
        }
        if j < lines.len() {
            break;
        }
        let Some(n) = return_or_goto(&blocks[u].terminator).and_then(|s| find(blocks, s)) else {
            return false;
        };
        if n == bi || blocks_naming(blocks, blocks[n].name) != 1 {
            return false;
        }
        u = n;
        j = 0;
    }
    let target: Line = blocks[u].each_line().nth(j).unwrap();
    if target.def_use().0 == Some(k) {
        return false;
    }
    if address_taken(blocks, &PlaceBase::Local(a)) {
        return false;
    }
    // two element reads in one statement would need two built instructions: keep the copy (a call is fine: it
    // stores each argument on its own)
    let call = matches!(target, Line::Terminator(Terminator::Call { func, target: Some(_), .. }) if one_word(func) && !func.is("out") && !func.is("digit"));
    if !call && reads_element(target) {
        return false;
    }
    // the first read of k takes the element
    let mut done = false;
    let take = &mut |o: &mut Operand| {
        if !done && o.local() == Some(k) {
            *o = Operand::Copy(el.clone());
            done = true;
        }
    };
    if j < blocks[u].statements.len() {
        blocks[u].statements[j].operands_mut().into_iter().for_each(take);
    } else {
        blocks[u].terminator.operands_mut().into_iter().for_each(take);
    }
    blocks[bi].statements.remove(i);
    true
}

/// A path of one word: a function of this program, or a built-in.
fn one_word(p: &Path) -> bool {
    p.qself.is_none() && matches!(p.segments.as_slice(), [Segment::Name { args: None, .. }])
}

/// Does what a line reads (past its destination) include an element at a computed index, `xs[i]` or `xs[i+k]`?
fn reads_element(l: Line) -> bool {
    let dynamic = |p: &Place| {
        p.projection
            .iter()
            .any(|q| matches!(q, Projection::Index(_) | Projection::IndexPlus(..)))
    };
    match l {
        Line::Statement(Statement::Assign(_, rv)) => {
            rv.operands().iter().filter_map(|o| o.place()).any(dynamic)
                || matches!(rv, Rvalue::Ref(_, p) | Rvalue::Discriminant(p) if dynamic(p))
        }
        Line::Statement(s) => s.places().into_iter().any(dynamic),
        Line::Terminator(Terminator::Call { args, .. }) => args.iter().filter_map(|o| o.place()).any(dynamic),
        Line::Terminator(t) => t.places().into_iter().any(dynamic),
    }
}

impl Line<'_> {
    fn count_of(self, l: Local) -> usize {
        self.locals().iter().filter(|x| **x == l).count()
    }
}

/// Field forwarding. An overflow-checked op leaves its result in (_t.0: T), and MIR moves it out at once:
/// `_x = move (_t.0: T)`. When that is the field's only read and x is only read as an operand in the same block, the
/// reads use the field and the move goes; when the op is right before (its check proven away), the op writes x.
pub fn forward_fields(blocks: &mut [Block], named: &HashSet<Local>) {
    for bi in 0..blocks.len() {
        let mut i: usize = 0;
        while i < blocks[bi].statements.len() {
            let Statement::Assign(d, Rvalue::Use(Operand::Move(fp))) = &blocks[bi].statements[i] else {
                i += 1;
                continue;
            };
            let (Some(x), Some(t), [Projection::Field(0, ty)]) = (
                d.as_local().filter(|l| *l != Local::UNIT),
                fp.base_local(),
                fp.projection.as_slice(),
            ) else {
                i += 1;
                continue;
            };
            if !one_word_ty(ty) {
                i += 1;
                continue;
            }
            let field = fp.clone();
            let lines_reading = |pred: &dyn Fn(Line) -> bool| -> usize {
                blocks
                    .iter()
                    .flat_map(|b| b.each_line())
                    .filter(|l| pred(*l) && line_places(*l).iter().any(|p| p.is_value_of(t)))
                    .count()
            };
            let field_uses = lines_reading(&|_| true);
            let opening = |l: Line| match l {
                Line::Statement(Statement::Assign(p, _)) => *p == field,
                Line::Terminator(Terminator::Call { dest, .. }) => *dest == field,
                _ => false,
            };
            let prev = i > 0 && opening(Line::Statement(&blocks[bi].statements[i - 1]));
            if prev && field_uses == 2 {
                if let Statement::Assign(p, _) = &mut blocks[bi].statements[i - 1] {
                    *p = Place::local(x);
                }
                blocks[bi].statements.remove(i);
                continue;
            }
            if named.contains(&x) || x == Local(0) {
                i += 1;
                continue;
            }
            let reads = lines_reading(&|l| !opening(l) && !matches!(l, Line::Terminator(Terminator::Assert { .. })));
            let users = lines_naming(blocks, x);
            let here: Vec<Line> = blocks[bi].each_line().skip(i + 1).collect();
            if reads != 1 || users != 1 + here.iter().filter(|l| l.locals().contains(&x)).count() {
                i += 1;
                continue;
            }
            let written = here.iter().any(|l| {
                l.def_use().0 == Some(x)
                    || line_places(*l).iter().any(|p| p.projection.iter().any(|q| matches!(q, Projection::Index(i) if *i == x)))
                    || matches!(l, Line::Statement(Statement::Assign(_, Rvalue::Ref(_, p))) if p.base == PlaceBase::Local(x) && p.wrapped() == 0)
            });
            if written {
                i += 1;
                continue;
            }
            blocks[bi].statements.remove(i);
            let read = &mut |o: &mut Operand| {
                if matches!(o, Operand::Copy(_) | Operand::Move(_)) && o.local() == Some(x) {
                    *o = Operand::Copy(field.clone());
                }
            };
            let b = &mut blocks[bi];
            for s in b.statements[i..].iter_mut() {
                s.operands_mut().into_iter().for_each(&mut *read);
            }
            b.terminator.operands_mut().into_iter().for_each(read);
        }
    }
}

/// Every place a line names.
fn line_places<'a>(l: Line<'a>) -> Vec<&'a Place> {
    match l {
        Line::Statement(s) => s.places(),
        Line::Terminator(t) => t.places(),
    }
}

/// A type printed as one word.
fn one_word_ty(t: &Ty) -> bool {
    match t {
        Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str => true,
        Ty::Path(p) => one_word(p),
        _ => false,
    }
}

/// What a local in a run of plain lines ends up holding, in terms of the values at the run's start.
#[derive(Clone, PartialEq)]
enum Val {
    Op(Operation, Vec<Val>),
    Copy(Local),
    Konst(Const),
}

#[derive(Clone, Copy, PartialEq)]
enum Operation {
    Binary(BinOp),
    Unary(UnOp),
}

impl Val {
    fn operand(&self) -> Operand {
        match self {
            Val::Copy(c) => Operand::Copy(Place::local(*c)),
            Val::Konst(k) => Operand::Constant(k.clone()),
            Val::Op(..) => unreachable!("an operation's arguments are operands"),
        }
    }
    fn rvalue(&self) -> Rvalue {
        match self {
            Val::Op(Operation::Binary(op), args) => Rvalue::BinaryOp(*op, args[0].operand(), args[1].operand()),
            Val::Op(Operation::Unary(op), args) => Rvalue::UnaryOp(*op, args[0].operand()),
            v => Rvalue::Use(v.operand()),
        }
    }
    fn reads(&self) -> Vec<Local> {
        match self {
            Val::Copy(c) => vec![*c],
            Val::Op(_, args) => args.iter().flat_map(Val::reads).collect(),
            Val::Konst(_) => vec![],
        }
    }
    fn swap(&self, x: Local, t: Local) -> Val {
        match self {
            Val::Copy(c) if *c == x => Val::Copy(t),
            Val::Op(op, args) => Val::Op(*op, args.iter().map(|a| a.swap(x, t)).collect()),
            v => v.clone(),
        }
    }
}

/// A plain line, as copies at a join work it out: `x = copy y`, `x = const c`, or an operation on locals and
/// constants. Gives the local it sets.
fn plain(s: &Statement) -> Option<Local> {
    let Statement::Assign(d, rv) = s else { return None };
    let x = d.as_local().filter(|l| *l != Local::UNIT)?;
    let arg = |o: &Operand| match o {
        Operand::Constant(c) => one_word_const(c),
        _ => o.local().is_some_and(|l| l != Local::UNIT),
    };
    let ok = match rv {
        Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => p.as_local().is_some_and(|l| l != Local::UNIT),
        Rvalue::Use(Operand::Constant(c)) => one_word_const(c),
        Rvalue::BinaryOp(_, a, b) => arg(a) && arg(b),
        Rvalue::UnaryOp(_, a) => arg(a),
        _ => false,
    };
    ok.then_some(x)
}

/// Copies at a join. `let (a, b) = if c { (y, x + 1) } else { (x + 2, y) }; x = a; y = b;` ends with copies in
/// the block both arms go to. Done in each arm instead, they can be worked out with the arm's own lines: in the
/// second arm, x ends up x + 2 and y stays y, so that arm is one line. A run of plain lines (copies, constants, one
/// operation on locals and constants) is worked out to what each local ends up holding, and written again in the
/// fewest lines, when that is fewer.
pub fn join_copies(blocks: &mut [Block], types: &HashMap<Local, Ty>, keep: &HashSet<Local>) {
    let addressed = addressed_locals(blocks);
    let ok = |x: Local| scalar(types, x) && !addressed.contains(&x);
    let copy_line = |s: &Statement| match s {
        Statement::Assign(d, Rvalue::Use(Operand::Copy(p) | Operand::Move(p))) => {
            matches!((d.as_local(), p.as_local()), (Some(x), Some(y)) if x != Local::UNIT && y != Local::UNIT && ok(x) && ok(y))
        }
        _ => false,
    };
    let simplify = |blocks: &mut [Block]| {
        let mut changed = true;
        while changed {
            changed = false;
            let live = live_after(blocks);
            for bi in 0..blocks.len() {
                let mut i = 0;
                while i < blocks[bi].statements.len() && !changed {
                    let ss = &blocks[bi].statements;
                    let mut j = i;
                    while j < ss.len() && plain(&ss[j]).is_some_and(ok) {
                        j += 1;
                    }
                    if j - i < 2 {
                        i += 1;
                        continue;
                    }
                    if let Some(done) = rewrite_run(&ss[i..j], &live[bi][j - 1], types, keep) {
                        blocks[bi].statements.splice(i..j, done);
                        changed = true;
                    }
                    i = j + 1;
                }
                if changed {
                    break;
                }
            }
        }
    };
    let size = |blocks: &[Block]| blocks.iter().map(|b| b.statements.len() + 1).sum::<usize>();
    simplify(blocks);
    // the copies that open a block go up into its predecessors, when every one of them only jumps there, and are
    // kept there when that leaves fewer lines in all
    for ji in 1..blocks.len() {
        let k = blocks[ji].statements.iter().take_while(|s| copy_line(s)).count();
        if k == 0 {
            continue;
        }
        let name = blocks[ji].name;
        let preds: Vec<usize> = (0..blocks.len())
            .filter(|&b| blocks[b].terminator.names(name))
            .collect();
        if preds.is_empty()
            || preds
                .iter()
                .any(|&b| b == ji || blocks[b].terminator != Terminator::Goto(name))
        {
            continue;
        }
        let was: Vec<Block> = blocks.to_vec();
        let before = size(blocks);
        let moved: Vec<Statement> = blocks[ji].statements.drain(0..k).collect();
        for &b in &preds {
            blocks[b].statements.extend(moved.iter().cloned());
        }
        simplify(blocks);
        if size(blocks) >= before {
            blocks.clone_from_slice(&was);
        }
    }
}

/// A run of plain lines, worked out and written again: each local set only once nothing still to be written needs
/// its old value. None when that is no shorter.
fn rewrite_run(
    run: &[Statement],
    live_out: &HashSet<Local>,
    types: &HashMap<Local, Ty>,
    keep: &HashSet<Local>,
) -> Option<Vec<Statement>> {
    let mut val: Ordered<Local, Val> = Ordered::new();
    let of = |val: &Ordered<Local, Val>, x: Local| val.get(&x).cloned().unwrap_or(Val::Copy(x));
    for s in run {
        let Statement::Assign(d, rv) = s else { unreachable!() };
        let x = d.as_local().unwrap();
        let arg = |val: &Ordered<Local, Val>, o: &Operand| match o {
            Operand::Constant(c) => Val::Konst(c.clone()),
            _ => of(val, o.local().unwrap()),
        };
        let v = match rv {
            Rvalue::Use(Operand::Constant(c)) => Val::Konst(c.clone()),
            Rvalue::Use(o) => of(&val, o.local().unwrap()),
            Rvalue::BinaryOp(op, a, b) => {
                let args = vec![arg(&val, a), arg(&val, b)];
                if args.iter().any(|v| matches!(v, Val::Op(..))) {
                    return None; // an operation on another's result: leave the run as it is
                }
                Val::Op(Operation::Binary(*op), args)
            }
            Rvalue::UnaryOp(op, a) => {
                let args = vec![arg(&val, a)];
                if matches!(args[0], Val::Op(..)) {
                    return None;
                }
                Val::Op(Operation::Unary(*op), args)
            }
            _ => unreachable!(),
        };
        val.set(x, v);
    }
    let mut todo: Vec<(Local, Val)> = val
        .iter()
        .filter(|(x, v)| (live_out.contains(x) || keep.contains(x)) && **v != Val::Copy(**x))
        .map(|(x, v)| (*x, v.clone()))
        .collect();
    let mut spare: Vec<Local> = val
        .keys()
        .filter(|x| !live_out.contains(x) && !keep.contains(x))
        .copied()
        .collect();
    let mut out = vec![];
    while !todo.is_empty() {
        let free = (0..todo.len()).find(|&n| {
            !todo
                .iter()
                .enumerate()
                .any(|(m, (_, v))| m != n && v.reads().contains(&todo[n].0))
        });
        if let Some(k) = free {
            let (x, v) = todo.remove(k);
            out.push(Statement::Assign(Place::local(x), v.rvalue()));
            continue;
        }
        // every local still to set is read by another: keep one old value in a local the run no longer needs
        let x = todo[0].0;
        let t = *spare
            .iter()
            .find(|s| types.get(s) == types.get(&x) && !todo.iter().any(|(y, v)| y == *s || v.reads().contains(s)))?;
        spare.retain(|s| *s != t);
        out.push(Statement::Assign(
            Place::local(t),
            Rvalue::Use(Operand::Copy(Place::local(x))),
        ));
        for (_, v) in todo.iter_mut() {
            *v = v.swap(x, t);
        }
    }
    if out.len() < run.len() {
        Some(out)
    } else {
        None
    }
}

/// Sharing cells. `let h = hour(inp())` makes rustc keep the card and the hour made from it in two locals, and
/// `if v < 0 { -v } else { v }` copies v into a third. When two locals are never needed at the same time, and one is
/// made from the other, they can be one cell: the copy disappears, and `x = x + 24` is done in place. Works on
/// scalar locals only, never on parameters or anything whose address is taken. A local can share a result's cell
/// (`results`, kept too), as when `Some(t)` ends a function that counts in t: the result is then worked in itself.
/// Gives each merge, in the order made: the local that went, and the one whose cell it now shares.
pub fn share_cells(
    blocks: &mut [Block],
    locals: &mut Ordered<Local, Ty>,
    keep: &HashSet<Local>,
    results: &HashSet<Local>,
    named: &HashSet<Local>,
) -> Vec<(Local, Local)> {
    let mut merged = vec![];
    let addressed = addressed_locals(blocks);
    for _round in 0..50 {
        let types: HashMap<Local, Ty> = locals.iter().map(|(l, t)| (*l, t.clone())).collect();
        let ok = |x: Local| scalar(&types, x) && !keep.contains(&x) && !addressed.contains(&x);
        let result = |x: Local| results.contains(&x) && scalar(&types, x) && !addressed.contains(&x);
        // the caller reads the results after the return
        let live = live_after_returning(blocks, results);
        // per line: what it defines, what is live after it, what it copies, what it reads
        struct LineInfo {
            def: Option<Local>,
            live: HashSet<Local>,
            copy: Option<Local>,
            uses: Vec<Local>,
        }
        let mut after: Vec<LineInfo> = vec![];
        for (bi, b) in blocks.iter().enumerate() {
            for (i, l) in b.each_line().enumerate() {
                let (def, uses) = l.def_use();
                let copy = match l {
                    Line::Statement(Statement::Assign(d, Rvalue::Use(Operand::Copy(p) | Operand::Move(p))))
                        if d.as_local().is_some_and(|x| x != Local::UNIT) =>
                    {
                        p.as_local()
                    }
                    _ => None,
                };
                after.push(LineInfo {
                    def,
                    live: live[bi][i].clone(),
                    copy,
                    uses,
                });
            }
        }
        let one_block = |x: Local| blocks.iter().filter(|b| b.lines().any(|l| l.mentions(x))).count() == 1;
        let from_call = |x: Local| {
            blocks.iter().any(|b| matches!(&b.terminator, Terminator::Call { dest, func, target: Some(_), .. } if dest.as_local() == Some(x) && one_word(func) && !func.is("inp")))
                && after.iter().filter(|y| y.uses.contains(&x)).count() == 1
        };
        let mut merge: Option<(Local, Local)> = None;
        'lines: for a in &after {
            let Some(def) = a.def else { continue };
            if !ok(def) && !result(def) {
                continue;
            }
            let mut srcs: Vec<Local> = vec![];
            for u in &a.uses {
                if !srcs.contains(u) {
                    srcs.push(*u);
                }
            }
            for src in srcs {
                if src == def || !ok(src) || types.get(&src) != types.get(&def) {
                    continue;
                }
                // an unnamed local used inside one block only already gets a shared scratch cell: merging it saves nothing
                if (one_block(src) && !named.contains(&src))
                    || (one_block(def) && !named.contains(&def) && !result(def))
                {
                    continue;
                }
                // a function's result read once is read straight from the function's own cell; giving it one adds a copy
                if from_call(src) || from_call(def) {
                    continue;
                }
                // never needed at once: wherever one is set, the other is not still to be read (a plain copy between them is fine)
                let clash = after.iter().any(|x| {
                    (x.def == Some(def) && x.live.contains(&src) && x.copy != Some(src))
                        || (x.def == Some(src) && x.live.contains(&def) && x.copy != Some(def))
                });
                if clash {
                    continue;
                }
                merge = Some((src, def));
                break 'lines;
            }
        }
        let Some((src, def)) = merge else { break };
        for b in blocks.iter_mut() {
            rename(b, src, &PlaceBase::Local(def));
            b.statements.retain(|s| !matches!(s, Statement::Assign(d, Rvalue::Use(Operand::Copy(p) | Operand::Move(p))) if d.as_local() == Some(def) && p.as_local() == Some(def)));
        }
        locals.remove(&src);
        merged.push((src, def));
    }
    merged
}

/// Parameters passed straight on. A parameter read once, as an argument of a call or of the multiply or divide
/// routine, can live in that argument's cell: the caller stores straight into it, and the copy and the parameter's
/// own cell go. That needs the cell to hold until the read, so the read has to be in the function's first block,
/// which nothing jumps back to, with no multiply or divide before it (they would use the routines' cells), and
/// calls only end a block. Gives each such parameter's cell and the cell it shares, through `alias` for a callee
/// whose own parameter is passed on in turn.
pub fn forwarded(
    func: &str,
    blocks: &[Block],
    params: &[Local],
    results: &HashSet<Local>,
    types: &HashMap<Local, Ty>,
    sigs: &Ordered<String, Sig>,
    alias: &HashMap<Sym, Sym>,
) -> Vec<(Sym, Sym)> {
    let mut out: Vec<(Sym, Sym)> = vec![];
    let Some(first) = blocks.first() else { return out };
    if blocks.iter().any(|b| b.terminator.names(first.name)) {
        return out;
    }
    let routine = |l: Line| {
        let op = match l {
            Line::Statement(Statement::Assign(_, Rvalue::BinaryOp(op, ..))) => Some(*op),
            _ => None,
        };
        matches!(
            op,
            Some(BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::MulWithOverflow | BinOp::MulWrapping)
        )
    };
    for &p in params {
        if results.contains(&p) || !scalar(types, p) {
            continue;
        }
        if occurrences(blocks, p) != 1 {
            continue;
        }
        let lines: Vec<Line> = first.each_line().collect();
        let Some(j) = lines.iter().position(|l| l.count_of(p) == 1) else {
            continue;
        };
        if lines[..j].iter().any(|l| routine(*l)) {
            continue;
        }
        let read = |o: &Operand| matches!(o, Operand::Copy(_) | Operand::Move(_)) && o.local() == Some(p);
        let mut to = None;
        match lines[j] {
            Line::Terminator(Terminator::Call {
                func: callee,
                args,
                target: Some(_),
                ..
            }) if one_word(callee) => {
                let name = callee.name();
                let builtin = [
                    "out",
                    "inp",
                    "overlay",
                    "digit",
                    "skip_cards",
                    "add_times",
                    "assert_unchecked",
                ]
                .contains(&name);
                if let (false, Some(sig)) = (builtin || name == func, sigs.get(name)) {
                    if let Some(i) = args.iter().position(read).filter(|_| args.len() == sig.params.len()) {
                        to = Some(Sym::local(name, sig.params[i]));
                    }
                }
            }
            Line::Statement(Statement::Assign(
                d,
                Rvalue::BinaryOp(op @ (BinOp::Mul | BinOp::MulWithOverflow | BinOp::Div | BinOp::Rem), a, b),
            )) => {
                // two locals: the routine, the first operand into its a and the second into its b (a divide only when
                // signed, as an unsigned one is a loop of its own)
                let both = [a, b].iter().all(|o| {
                    matches!(o, Operand::Copy(_) | Operand::Move(_)) && o.local().is_some_and(|l| l != Local::UNIT)
                });
                if let (Some(m1), true) = (d.as_local().filter(|l| *l != Local::UNIT), both) {
                    let first = read(a);
                    let cell = if matches!(op, BinOp::Mul | BinOp::MulWithOverflow) {
                        Some(if first { "rt_mul_a" } else { "rt_mul_b" })
                    } else if types.get(&m1) == Some(&Ty::Int(IntTy::I16)) {
                        Some(if first { "rt_div_a" } else { "rt_div_b" })
                    } else {
                        None
                    };
                    to = cell.map(Sym::Runtime);
                }
            }
            _ => {}
        }
        let Some(to) = to else { continue };
        let to = alias.get(&to).cloned().unwrap_or(to);
        if out.iter().any(|(_, t)| *t == to) {
            continue;
        }
        out.push((Sym::local(func, p), to));
    }
    out
}
