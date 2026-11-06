//! For loops. `for i in a..b` (or `a..=b`, or either `.rev()`) reaches the MIR as calls into the standard library:
//! a Range built, then `next()` called until it returns None. Those calls are turned back into the counting loop
//! they stand for, before anything else sees them. The range becomes a two-field struct (start, end), which the
//! struct pass then splits into two cells, and each `next()` becomes a test and a step:
//!   a..b          start < end, then i = start, start += 1
//!   a..=b         start <= end, the same step (so an end of 999 is out of reach: start would pass a card)
//!   (a..b).rev()  start < end, then end -= 1, i = end
//!   (a..=b).rev() start <= end, then i = end, end -= 1
//! At opt-level 2 rustc has already inlined a..b into a counter, but in its own shape, with an overflow check it
//! knows can't fire and an Option it unwraps at once; those are cleaned up here too. Last, where the loop's value is
//! a copy taken before the counter steps, the step moves to the end of the loop body, so the counter itself is the
//! value: no extra cell, the same code as `while i < n { ...; i += 1; }`.
use std::collections::HashSet;

use super::ir::{
    BasicBlock, BinOp, Block, Body, Const, Decl, GenericArg, IntTy, Local, Operand, Path, Place, PlaceBase, Projection,
    Rvalue, Segment, Statement, Terminator, Ty,
};
use super::structs::Function;
use super::util::Ordered;

/// A range type: inclusive or not, reversed or not, and its element type.
#[derive(Clone, Copy)]
struct RangeTy {
    incl: bool,
    rev: bool,
    t: IntTy,
}

/// `core::ops::Range<T>` or `core::ops::RangeInclusive<T>` for a card's type T: inclusive or not, and T.
fn core_range(p: &Path) -> Option<(bool, IntTy)> {
    let [Segment::Name { name: a, args: None }, Segment::Name { name: b, args: None }, Segment::Name { name, args: Some(g) }] =
        p.segments.as_slice()
    else {
        return None;
    };
    if p.qself.is_some() || a != "core" || b != "ops" || (name != "Range" && name != "RangeInclusive") {
        return None;
    }
    let [GenericArg::Ty(Ty::Int(t @ (IntTy::I16 | IntTy::U8 | IntTy::Usize)))] = g.args.as_slice() else {
        return None;
    };
    Some((name == "RangeInclusive", *t))
}

fn range(ty: Option<&Ty>) -> Option<RangeTy> {
    let Some(Ty::Path(p)) = ty else { return None };
    if let Some((incl, t)) = core_range(p) {
        return Some(RangeTy { incl, rev: false, t });
    }
    // Rev<core::ops::Range<T>>, or core::iter::Rev<..>
    let rev_seg = match p.segments.as_slice() {
        [s] => Some(s),
        [Segment::Name { name: a, args: None }, Segment::Name { name: b, args: None }, s]
            if a == "core" && b == "iter" =>
        {
            Some(s)
        }
        _ => None,
    };
    let Some(Segment::Name { name, args: Some(g) }) = rev_seg else {
        return None;
    };
    if name != "Rev" || p.qself.is_some() {
        return None;
    }
    let [GenericArg::Ty(Ty::Path(inner))] = g.args.as_slice() else {
        return None;
    };
    core_range(inner).map(|(incl, t)| RangeTy { incl, rev: true, t })
}

fn struct_name(r: &RangeTy) -> String {
    format!("{}_{}", if r.incl { "RangeInclusive" } else { "Range" }, r.t)
}

/// The struct a range local becomes, as a type.
fn struct_ty(r: &RangeTy) -> Ty {
    Ty::Path(Path {
        qself: None,
        segments: vec![Segment::Name {
            name: struct_name(r),
            args: None,
        }],
    })
}

/// The range literal `R { start: a, end: b }` for range struct R.
fn literal(r: &RangeTy, start: Operand, end: Operand) -> Rvalue {
    let p = Path {
        qself: None,
        segments: vec![Segment::Name {
            name: struct_name(r),
            args: None,
        }],
    };
    Rvalue::Struct(p, vec![("start".into(), start), ("end".into(), end)])
}

/// A range literal's start and end: its first two fields, `start` and `end` (an `exhausted` after them aside).
fn fields(fs: &[(String, Operand)]) -> Option<[Operand; 2]> {
    match fs {
        [(s, a), (e, b)] | [(s, a), (e, b), _] if s == "start" && e == "end" => Some([a.clone(), b.clone()]),
        _ => None,
    }
}

/// One line of a block, as the pass reads it.
#[derive(Clone, Copy)]
enum Line<'a> {
    S(&'a Statement),
    T(&'a Terminator),
}

impl Block {
    fn lines_of(&self) -> Vec<Line<'_>> {
        self.statements
            .iter()
            .map(Line::S)
            .chain(std::iter::once(Line::T(&self.terminator)))
            .collect()
    }
}

impl Line<'_> {
    /// The place it assigns: a statement's, or a call's destination.
    fn dest(&self) -> Option<&Place> {
        match self {
            Line::S(Statement::Assign(p, _)) => Some(p),
            Line::T(Terminator::Call { dest, .. }) => Some(dest),
            _ => None,
        }
    }

    fn places(&self) -> Vec<&Place> {
        match self {
            Line::S(s) => s.places(),
            Line::T(t) => t.places(),
        }
    }

    fn operands(&self) -> Vec<&Operand> {
        match self {
            Line::S(s) => s.operands(),
            Line::T(t) => t.operands(),
        }
    }
}

/// Every place a place names: itself, and the places its indexes are read from.
fn nested<'a>(p: &'a Place, out: &mut Vec<&'a Place>) {
    out.push(p);
    for q in &p.projection {
        if let Projection::IndexPlace(inner) = q {
            nested(inner, out);
        }
    }
}

/// How many times a line names `l` other than as a struct whose field it reads (`(_l.0: T)` names the field).
fn mentions(line: Line, l: Local) -> usize {
    fn place(p: &Place, l: Local) -> usize {
        let base =
            (p.base == PlaceBase::Local(l) && !matches!(p.projection.first(), Some(Projection::Field(..)))) as usize;
        base + p
            .projection
            .iter()
            .map(|q| match q {
                Projection::Index(i) | Projection::IndexPlus(i, _) => (*i == l) as usize,
                Projection::IndexPlace(inner) => place(inner, l),
                _ => 0,
            })
            .sum::<usize>()
    }
    match line {
        Line::S(Statement::Unsupported(u)) | Line::T(Terminator::Unsupported(u)) => {
            u.locals.iter().filter(|x| **x == l).count()
        }
        Line::S(Statement::StorageLive(x) | Statement::StorageDead(x)) => (*x == l) as usize,
        _ => {
            let mut n = 0;
            for p in line.places() {
                n += place(p, l);
            }
            // a place a reference is taken of (not an operand) is among the places too; an index local of an
            // operand's place is counted with that place
            n
        }
    }
}

impl Body {
    fn all_lines(&self) -> impl Iterator<Item = Line<'_>> {
        self.blocks.iter().flat_map(|b| b.lines_of())
    }

    /// How many times `l` is named, a struct's field reads aside.
    fn uses(&self, l: Local) -> usize {
        self.all_lines().map(|x| mentions(x, l)).sum()
    }

    /// How many lines assign `x` whole.
    fn sets(&self, x: Local) -> usize {
        self.all_lines()
            .filter(|l| l.dest().and_then(Place::as_local) == Some(x))
            .count()
    }

    /// Does any place read field k of `x` first, `(x.k: T)` (with type `ty`, when given)?
    fn field_places(&self, x: Local, k: u32, ty: Option<&Ty>) -> usize {
        let mut n = 0;
        for line in self.all_lines() {
            for p in line.places() {
                let mut all = vec![];
                nested(p, &mut all);
                for q in all {
                    if q.base == PlaceBase::Local(x)
                        && matches!(q.projection.first(), Some(Projection::Field(j, t)) if *j == k && ty.is_none_or(|ty| t == ty))
                    {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    /// Is the address of `x` taken, of the local itself or at an index?
    fn taken(&self, x: Local) -> bool {
        self.all_lines().any(|l| matches!(l, Line::S(Statement::Assign(_, Rvalue::Ref(_, p))) if p.base == PlaceBase::Local(x) && p.wrapped() == 0))
    }

    fn at(&self, name: BasicBlock) -> Option<usize> {
        self.blocks.iter().rposition(|b| b.name == name)
    }

    fn block(&self, name: BasicBlock) -> Option<&Block> {
        self.at(name).map(|i| &self.blocks[i])
    }

    /// The blocks whose terminator names `x`.
    fn preds(&self, x: BasicBlock) -> Vec<BasicBlock> {
        self.blocks
            .iter()
            .filter(|y| y.terminator.successors().contains(&x))
            .map(|y| y.name)
            .collect()
    }

    /// The blocks reached from bb0 without passing through `avoid`.
    fn reached(&self, avoid: Option<BasicBlock>) -> HashSet<BasicBlock> {
        let entry = BasicBlock(0);
        let mut seen: HashSet<BasicBlock> = HashSet::from([entry]);
        let mut work = vec![entry];
        while let Some(n) = work.pop() {
            let Some(b) = self.block(n) else { continue };
            for s in b.terminator.successors() {
                if Some(s) != avoid && !seen.contains(&s) {
                    seen.insert(s);
                    work.push(s);
                }
            }
        }
        seen
    }

    /// Every place, mutably.
    fn each_place(&mut self, f: &mut dyn FnMut(&mut Place)) {
        for b in self.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.places_mut(f);
            }
            b.terminator.places_mut(f);
        }
    }

    /// `((x as Some).0: T)` becomes x itself, wherever it is read.
    fn unwrap_some(&mut self, x: Local, t: IntTy) {
        let field = Projection::Field(0, Ty::Int(t));
        self.each_place(&mut |p| {
            fn go(p: &mut Place, x: Local, field: &Projection) {
                for q in p.projection.iter_mut() {
                    if let Projection::IndexPlace(inner) = q {
                        go(inner, x, field);
                    }
                }
                if p.base == PlaceBase::Local(x)
                    && p.projection.len() >= 2
                    && p.projection[0] == Projection::Downcast("Some".into())
                    && p.projection[1] == *field
                {
                    p.projection.drain(0..2);
                }
            }
            go(p, x, &field)
        });
    }

    /// Name the place `to` wherever `v` is named, other than as a struct whose field is read.
    fn replace_word(&mut self, v: Local, to: &Place) {
        fn go(p: &mut Place, v: Local, to: &Place) {
            for q in p.projection.iter_mut() {
                match q {
                    Projection::Index(i) | Projection::IndexPlus(i, _) if *i == v => match to.as_local() {
                        Some(l) => *i = l,
                        None if matches!(q, Projection::Index(_)) => *q = Projection::IndexPlace(Box::new(to.clone())),
                        None => {}
                    },
                    Projection::IndexPlace(inner) => go(inner, v, to),
                    _ => {}
                }
            }
            if p.base == PlaceBase::Local(v) && !matches!(p.projection.first(), Some(Projection::Field(..))) {
                let rest = std::mem::take(&mut p.projection);
                *p = Place {
                    base: to.base.clone(),
                    projection: to.projection.iter().cloned().chain(rest).collect(),
                };
            }
        }
        self.each_place(&mut |p| go(p, v, to));
    }

    /// A debug name of `v` names `to` instead, when `to` is a plain local.
    fn rename_debug(&mut self, v: Local, to: &Place) {
        let Some(t) = to.as_local() else { return };
        for d in self.debug.iter_mut() {
            if d.named_local().is_some_and(|(_, l)| l == v) {
                d.value = t.to_string();
            }
        }
    }
}

/// `x = copy p;` for a whole local x: x and p.
fn copy_into(s: &Statement) -> Option<(Local, &Place)> {
    match s {
        Statement::Assign(d, Rvalue::Use(Operand::Copy(p))) => {
            d.as_local().filter(|l| *l != Local::UNIT).map(|l| (l, p))
        }
        _ => None,
    }
}

/// `c = Add|Sub(copy from, const 1_T);` with c the given place and `from` the given place: the operator and the one.
fn step_of(l: Line, c: &Place, from: &Place) -> Option<(BinOp, Operand)> {
    let Line::S(Statement::Assign(d, Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub), Operand::Copy(a), one))) = l
    else {
        return None;
    };
    (d == c && a == from && matches!(one.int(), Some((1, _)))).then(|| (*op, one.clone()))
}

pub fn lower_for_loops(f: &mut Function) {
    if f.body.blocks.is_empty() {
        return;
    }
    let mut bd = f.body.clone();
    let mut decl: Ordered<Local, Ty> = Ordered::new();
    for d in &bd.decls {
        decl.set(d.local, d.ty.clone());
    }
    for (l, t) in &f.params {
        decl.set(*l, t.clone());
    }
    let mut retype: Ordered<Local, Ty> = Ordered::new(); // local -> its new type
    let mut next_local = decl.keys().map(|l| l.0).max().unwrap_or(0) + 1;
    let mut next_block = bd.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let mut loops: Vec<(BasicBlock, BasicBlock)> = vec![]; // each loop's test block, and the block after it that steps
    let is_arg = |x: Local| f.params.iter().any(|(l, _)| *l == x);
    // building and copying the range
    let mut found = false;
    for b in bd.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            let Statement::Assign(d, rv) = s else { continue };
            let Some(x) = d.as_local().filter(|l| *l != Local::UNIT) else {
                continue;
            };
            let new = match rv {
                Rvalue::Struct(p, fs) => match (core_range(p), fields(fs)) {
                    (Some((incl, t)), Some([a, e])) => Some(literal(&RangeTy { incl, rev: false, t }, a, e)),
                    _ => match (rev_of_range(p), fs.as_slice()) {
                        (true, [(name, iter)]) if name == "iter" => Some(Rvalue::Use(iter.clone())),
                        _ => None,
                    },
                },
                Rvalue::BinaryOp(op @ (BinOp::AddUnchecked | BinOp::SubUnchecked), a, c) => Some(Rvalue::BinaryOp(
                    if *op == BinOp::AddUnchecked {
                        BinOp::Add
                    } else {
                        BinOp::Sub
                    },
                    a.clone(),
                    c.clone(),
                )),
                _ => None,
            };
            if let Some(r) = new {
                found = true;
                *s = Statement::Assign(Place::local(x), r);
            }
        }
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(target),
            ..
        } = &b.terminator
        else {
            continue;
        };
        let Some(x) = dest.as_local().filter(|l| *l != Local::UNIT) else {
            continue;
        };
        let target = *target;
        // into_iter() and rev() pass the range through
        if let (Some(q), [Segment::Name { name: m, args: None }]) = (&func.qself, func.segments.as_slice()) {
            let iter_trait = q.as_trait.as_ref().is_some_and(|t| {
                t.qself.is_none()
                    && t.segments.iter().all(|s| matches!(s, Segment::Name { args: None, .. }))
                    && (t.name() == "IntoIterator" || t.name() == "Iterator")
            });
            if iter_trait && (m == "into_iter" || m == "rev") && range(Some(&q.ty)).is_some() && args.len() == 1 {
                found = true;
                let built = match &args[0] {
                    Operand::Constant(Const::Adt(p, cf)) => {
                        let (incl, t) = core_range(p).expect("a range of an unsupported type");
                        let get = |n: &str| {
                            cf.iter()
                                .find(|(k, _)| k == n)
                                .map(|(_, v)| Operand::Constant(v.clone()))
                                .expect("a range constant without its start and end")
                        };
                        literal(&RangeTy { incl, rev: false, t }, get("start"), get("end"))
                    }
                    a => Rvalue::Use(a.clone()),
                };
                b.statements.push(Statement::Assign(Place::local(x), built));
                b.terminator = Terminator::Goto(target);
                continue;
            }
        }
        // RangeInclusive::new(a, b) builds it
        if let (
            [Segment::Name { name: c, args: None }, Segment::Name { name: o, args: None }, Segment::Name { name: r, args: Some(g) }, Segment::Name { name: new, args: None }],
            [a, e],
        ) = (func.segments.as_slice(), args.as_slice())
        {
            if func.qself.is_none() && c == "core" && o == "ops" && r == "RangeInclusive" && new == "new" {
                if let [GenericArg::Ty(Ty::Int(t @ (IntTy::I16 | IntTy::U8 | IntTy::Usize)))] = g.args.as_slice() {
                    found = true;
                    let lit = literal(
                        &RangeTy {
                            incl: true,
                            rev: false,
                            t: *t,
                        },
                        a.clone(),
                        e.clone(),
                    );
                    b.statements.push(Statement::Assign(Place::local(x), lit));
                    b.terminator = Terminator::Goto(target);
                }
            }
        }
    }
    // each next(): a test, and a step into the loop body
    let names: Vec<BasicBlock> = bd.blocks.iter().map(|b| b.name).collect();
    for name in names {
        let bi = bd.at(name).unwrap();
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(rb),
            ..
        } = &bd.blocks[bi].terminator
        else {
            continue;
        };
        let methods = ["next", "spec_next", "next_back", "spec_next_back"];
        let (Some(r_), true, [Operand::Move(pp)]) = (
            dest.as_local().filter(|l| *l != Local::UNIT),
            func.qself.is_some() && func.segments.len() == 1 && methods.contains(&func.name()),
            args.as_slice(),
        ) else {
            continue;
        };
        let (Some(p), rb, method) = (pp.as_local(), *rb, func.name().to_string()) else {
            continue;
        };
        let Some(pi) = bd.blocks[bi].statements.iter().position(|s| matches!(s, Statement::Assign(d, Rvalue::Ref(super::ir::BorrowKind::Mut, _)) if d.as_local() == Some(p))) else { continue };
        let Statement::Assign(_, Rvalue::Ref(_, borrowed)) = &bd.blocks[bi].statements[pi] else {
            unreachable!()
        };
        let it = match borrowed.projection.as_slice() {
            [] => borrowed.base_local(),
            // the range inside a Rev: `&mut (_13.0: core::ops::RangeInclusive<i16>)`
            [Projection::Field(0, t)] if range(Some(t)).is_some() => borrowed.base_local(),
            _ => None,
        };
        let Some(it) = it else { continue };
        let Some(r) = range(decl.get(&it)) else { continue };
        let Some(rbk) = bd.block(rb) else { continue };
        if rbk.statements.len() != 1 {
            continue;
        }
        let Statement::Assign(sd, Rvalue::Discriminant(sp)) = &rbk.statements[0] else {
            continue;
        };
        let (Some(sw0), Some(sw1)) = (sd.as_local(), sp.as_local()) else {
            continue;
        };
        if sw1 != r_ {
            continue;
        }
        let Terminator::SwitchInt {
            discr: Operand::Move(st0),
            arms,
            otherwise: Some(_),
        } = &rbk.terminator
        else {
            continue;
        };
        let [(0, done), (1, body)] = arms.as_slice() else {
            continue;
        };
        if st0.as_local() != Some(sw0) {
            continue;
        }
        let (done, body) = (*done, *body);
        let rev = r.rev || method.ends_with("back");
        let t = r.t;
        let one = Operand::Constant(Const::Int { value: 1, ty: t });
        let (s, e) = (Place::field(it, 0, Ty::Int(t)), Place::field(it, 1, Ty::Int(t)));
        let c = Local(next_local);
        next_local += 1;
        retype.set(c, Ty::Bool);
        let step = BasicBlock(next_block);
        next_block += 1;
        let b = &mut bd.blocks[bi];
        b.statements.remove(pi);
        let cmp = if r.incl { BinOp::Le } else { BinOp::Lt };
        b.statements.push(Statement::Assign(
            Place::local(c),
            Rvalue::BinaryOp(cmp, Operand::Copy(s.clone()), Operand::Copy(e.clone())),
        ));
        b.terminator = Terminator::SwitchInt {
            discr: Operand::Move(Place::local(c)),
            arms: vec![(0, done)],
            otherwise: Some(step),
        };
        let assign = |p: &Place, r: Rvalue| Statement::Assign(p.clone(), r);
        let rl = Place::local(r_);
        let mut statements = if !rev {
            vec![
                assign(&rl, Rvalue::Use(Operand::Copy(s.clone()))),
                assign(&s, Rvalue::BinaryOp(BinOp::Add, Operand::Copy(rl.clone()), one.clone())),
            ]
        } else if r.incl {
            vec![
                assign(&rl, Rvalue::Use(Operand::Copy(e.clone()))),
                assign(&e, Rvalue::BinaryOp(BinOp::Sub, Operand::Copy(rl.clone()), one.clone())),
            ]
        } else {
            vec![
                assign(&e, Rvalue::BinaryOp(BinOp::Sub, Operand::Copy(e.clone()), one.clone())),
                assign(&rl, Rvalue::Use(Operand::Copy(e.clone()))),
            ]
        };
        statements.shrink_to_fit();
        bd.blocks.insert(
            bi + 1,
            Block {
                name: step,
                statements,
                terminator: Terminator::Goto(body),
            },
        );
        loops.push((name, step));
        retype.set(r_, Ty::Int(t));
        retype.set(p, Ty::Unit);
        retype.set(sw0, Ty::Unit);
        bd.unwrap_some(r_, t);
        found = true;
    }
    // A range copied from local to local (into_iter, rev, a let) is built where it ends up. An end that never
    // changes is read where it came from: a constant, or a local set only once.
    let ranges: Vec<Local> = decl
        .iter()
        .filter(|(_, t)| range(Some(t)).is_some())
        .map(|(l, _)| *l)
        .collect();
    let mut field_cells: Vec<Local> = vec![];
    let mut changed = true;
    while changed {
        changed = false;
        'scan: for bi in 0..bd.blocks.len() {
            for i in 0..bd.blocks[bi].statements.len() {
                let Statement::Assign(d, Rvalue::Use(Operand::Copy(src) | Operand::Move(src))) =
                    &bd.blocks[bi].statements[i]
                else {
                    continue;
                };
                let (Some(to), Some(a)) = (d.as_local(), src.as_local()) else {
                    continue;
                };
                if !ranges.contains(&to) || !ranges.contains(&a) {
                    continue;
                }
                if bd.uses(a) != 2 {
                    continue;
                }
                let is_def = |s: &Statement| matches!(s, Statement::Assign(d, Rvalue::Struct(_, fs)) if d.as_local() == Some(a) && fs.first().is_some_and(|(n, _)| n == "start"));
                let Some(def) = bd
                    .blocks
                    .iter()
                    .flat_map(|x| x.statements.iter())
                    .find(|s| is_def(s))
                    .cloned()
                else {
                    continue;
                };
                let copy_line = bd.blocks[bi].statements[i].clone();
                let Statement::Assign(_, built) = &def else {
                    unreachable!()
                };
                let moved = Statement::Assign(Place::local(to), built.clone());
                for x in bd.blocks.iter_mut() {
                    x.statements = x
                        .statements
                        .iter()
                        .filter(|s| **s != def)
                        .map(|s| if *s == copy_line { moved.clone() } else { s.clone() })
                        .collect();
                }
                changed = true;
                break 'scan;
            }
        }
    }
    for it in &ranges {
        let it = *it;
        let all = bd.clone();
        let t = range(decl.get(&it)).unwrap().t;
        let defs: Vec<&Statement> = all
            .blocks
            .iter()
            .flat_map(|b| b.statements.iter())
            .filter(|s| s.def() == Some(it))
            .collect();
        let call_defs = all.blocks.iter().filter(|b| b.terminator.def() == Some(it)).count();
        if defs.len() + call_defs != 1 || defs.len() != 1 {
            continue;
        }
        let Statement::Assign(_, Rvalue::Struct(_, fs)) = defs[0] else {
            continue;
        };
        let Some(ops) = fields(fs) else { continue };
        let opening = |l: Line| l.dest().and_then(Place::as_local) == Some(it);
        for (k, op) in ops.iter().enumerate() {
            let proj = Place::field(it, k as u32, Ty::Int(t));
            let x = match op {
                Operand::Copy(p) | Operand::Move(p) => p.as_local().filter(|l| *l != Local::UNIT),
                _ => None,
            };
            let sets_once = |x: Local| all.sets(x) == 1;
            // a local read only to build the range: its cell can be the field
            if let Some(x) = x {
                if all.uses(x) == 2 && sets_once(x) && !is_arg(x) && decl.get(&x) == Some(&Ty::Int(t)) {
                    let to = Place::local(x);
                    for b in bd.blocks.iter_mut() {
                        let mut swap = |p: &mut Place| field_to(p, &proj, &to);
                        for s in b.statements.iter_mut() {
                            if !opening(Line::S(s)) {
                                s.places_mut(&mut swap);
                            }
                        }
                        if !opening(Line::T(&b.terminator)) {
                            b.terminator.places_mut(&mut swap);
                        }
                    }
                    if !field_cells.contains(&x) {
                        field_cells.push(x);
                    }
                    continue;
                }
            }
            if all.all_lines().any(|l| l.dest() == Some(&proj)) {
                continue;
            }
            let single = x.is_some_and(|x| sets_once(x) && !is_arg(x) && !all.taken(x));
            let konst = matches!(op, Operand::Constant(_));
            if !konst && !single {
                continue;
            }
            let to = if konst {
                op.clone()
            } else {
                Operand::Copy(Place::local(x.unwrap()))
            };
            for b in bd.blocks.iter_mut() {
                for s in b.statements.iter_mut() {
                    if opening(Line::S(s)) {
                        if let Some(x) = x {
                            first_move_to_copy(s.operands_mut(), x);
                        }
                    } else {
                        for o in s.operands_mut() {
                            read_to(o, &proj, &to);
                        }
                    }
                }
                if opening(Line::T(&b.terminator)) {
                    if let Some(x) = x {
                        first_move_to_copy(b.terminator.operands_mut(), x);
                    }
                } else {
                    for o in b.terminator.operands_mut() {
                        read_to(o, &proj, &to);
                    }
                }
            }
        }
    }
    // a range local's own type: the struct it now is
    for (l, ty) in decl.iter() {
        if let Some(r) = range(Some(ty)) {
            retype.set(*l, struct_ty(&r));
        }
    }
    // opt-level 2's inlined counters. A branch into a block that can't be reached (`unreachable`, or a cold path
    // into one) is dropped; an overflow check whose flag then goes unread is a plain add; an Option made and
    // unwrapped at once is its value.
    let dead_end = |bd: &Body, name: BasicBlock| -> bool {
        let Some(x) = bd.block(name) else { return false };
        if !x.statements.is_empty() {
            return false;
        }
        if x.terminator == Terminator::Unreachable {
            return true;
        }
        let unreachable = |next: &BasicBlock| {
            bd.block(*next)
                .is_some_and(|y| y.statements.is_empty() && y.terminator == Terminator::Unreachable)
        };
        // a cold path, as the front end leaves it: a jump on
        if let Terminator::Goto(next) = &x.terminator {
            return unreachable(next);
        }
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(next),
            ..
        } = &x.terminator
        else {
            return false;
        };
        if dest.as_local().is_none() || !args.is_empty() || !func.is_path(&["core", "intrinsics", "cold_path"]) {
            return false;
        }
        unreachable(next)
    };
    for bi in 0..bd.blocks.len() {
        let Terminator::SwitchInt { discr, arms, otherwise } = &bd.blocks[bi].terminator else {
            continue;
        };
        // the arms, `otherwise` last
        let all_arms: Vec<(Option<i128>, BasicBlock)> = arms
            .iter()
            .map(|(v, b)| (Some(*v), *b))
            .chain(otherwise.map(|o| (None, o)))
            .collect();
        let live: Vec<(Option<i128>, BasicBlock)> =
            all_arms.iter().copied().filter(|(_, b)| !dead_end(&bd, *b)).collect();
        if live.len() == all_arms.len() || live.is_empty() {
            continue;
        }
        found = true;
        let mut targets: Vec<BasicBlock> = vec![];
        for (_, b) in &live {
            if !targets.contains(b) {
                targets.push(*b);
            }
        }
        let discr = discr.clone();
        bd.blocks[bi].terminator = if targets.len() == 1 {
            Terminator::Goto(live[0].1)
        } else {
            let n = live.len();
            let arms = live[..n - 1]
                .iter()
                .map(|(v, b)| (v.expect("only the last arm is otherwise"), *b))
                .collect();
            Terminator::SwitchInt {
                discr,
                arms,
                otherwise: Some(live[n - 1].1),
            }
        };
    }
    if !found {
        return;
    }
    let mut changed = true;
    while changed {
        changed = false;
        let all = bd.clone();
        for bi in 0..bd.blocks.len() {
            for i in 0..bd.blocks[bi].statements.len() {
                let s = bd.blocks[bi].statements[i].clone();
                let Statement::Assign(d, rv) = &s else { continue };
                let Some(x) = d.as_local().filter(|l| *l != Local::UNIT) else {
                    continue;
                };
                // a flag copied out and never read
                if let Rvalue::Use(Operand::Copy(p)) = rv {
                    if p.base_local().is_some() && p.projection == [Projection::Field(1, Ty::Bool)] && all.uses(x) == 1
                    {
                        bd.blocks[bi].statements.remove(i);
                        retype.set(x, Ty::Unit);
                        changed = true;
                        break;
                    }
                }
                // an overflow check nobody reads: a plain add, straight into where its result is copied
                if let Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow), a, c) = rv {
                    if all.field_places(x, 1, Some(&Ty::Bool)) == 0 {
                        let plain = if *op == BinOp::AddWithOverflow {
                            BinOp::Add
                        } else {
                            BinOp::Sub
                        };
                        let b = &bd.blocks[bi];
                        let next = b.lines_of().get(i + 1).copied();
                        let n = match next {
                            Some(Line::S(Statement::Assign(nd, Rvalue::Use(Operand::Copy(q)))))
                                if q.base_local() == Some(x)
                                    && matches!(q.projection.as_slice(), [Projection::Field(0, t)] if one_word(t)) =>
                            {
                                nd.as_local()
                            }
                            _ => None,
                        };
                        if let (Some(n), 1) = (n, all.field_places(x, 0, None)) {
                            bd.blocks[bi].statements.splice(
                                i..i + 2,
                                [Statement::Assign(
                                    Place::local(n),
                                    Rvalue::BinaryOp(plain, a.clone(), c.clone()),
                                )],
                            );
                            retype.set(x, Ty::Unit);
                        } else {
                            let w = match decl.get(&x) {
                                Some(Ty::Tuple(ts)) if ts.len() == 2 && ts[1] == Ty::Bool && one_word(&ts[0]) => {
                                    ts[0].clone()
                                }
                                Some(t) => t.clone(),
                                None => Ty::Other(String::new()),
                            };
                            bd.blocks[bi].statements[i] =
                                Statement::Assign(Place::field(x, 0, w), Rvalue::BinaryOp(plain, a.clone(), c.clone()));
                        }
                        changed = true;
                        break;
                    }
                }
                // Some(x), unwrapped at once
                if let Rvalue::Variant(p, vargs) = rv {
                    if let (Some(t), [v]) = (option_some(p), vargs.as_slice()) {
                        let discriminant = all.all_lines().any(|l| matches!(l, Line::S(Statement::Assign(_, Rvalue::Discriminant(q))) if q.as_local() == Some(x)));
                        // and read only as its value: not the result, nor passed, copied or set whole (an Option a
                        // function returns, or one `?` takes apart, is the enum pass's)
                        let whole =
                            |o: &Operand| matches!(o, Operand::Copy(q) | Operand::Move(q) if q.as_local() == Some(x));
                        let elsewhere = x == Local(0)
                            || is_arg(x)
                            || all.all_lines().any(|l| match l {
                                Line::S(s) => s.operands().into_iter().any(whole),
                                Line::T(t) => {
                                    t.operands().into_iter().any(whole)
                                        || matches!(t, Terminator::Call { dest, .. } if dest.as_local() == Some(x))
                                }
                            });
                        if !discriminant && !elsewhere {
                            bd.blocks[bi].statements[i] = Statement::Assign(Place::local(x), Rvalue::Use(v.clone()));
                            retype.set(x, Ty::Int(t));
                            bd.unwrap_some(x, t);
                            // and a constant `Some` it is set to elsewhere, its value (`Some(Equal)`, a comparison's)
                            for s in bd.blocks.iter_mut().flat_map(|b| b.statements.iter_mut()) {
                                if let Statement::Assign(d, Rvalue::Use(Operand::Constant(Const::Variant(p, cs)))) = s {
                                    if let (true, Some(_), [c]) =
                                        (d.as_local() == Some(x), option_some(p), cs.as_slice())
                                    {
                                        *s = Statement::Assign(d.clone(), Rvalue::Use(Operand::Constant(c.clone())));
                                    }
                                }
                            }
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
    }
    // blocks nothing reaches any more
    let reach = bd.reached(None);
    bd.blocks.retain(|b| reach.contains(&b.name));
    // The step moved to the end of the body, where the loop's value is a copy of the counter taken before it steps,
    // or where nothing in the body reads the counter at all (`for _ in 0..n`).
    let mut counters: Vec<Place> = vec![];
    let add = |counters: &mut Vec<Place>, p: Place| {
        if !counters.contains(&p) {
            counters.push(p);
        }
    };
    for d in &bd.debug {
        let iter = d.name.trim_start_matches('(');
        if d.name.starts_with('(') && iter.starts_with("iter: ") && d.name.ends_with(')') {
            if let Some(l) = Local::parse(&d.value).filter(|l| *l != Local::UNIT) {
                add(&mut counters, Place::local(l));
            }
        }
    }
    for (l, ty) in decl.iter() {
        if let Some(r) = range(Some(ty)) {
            for k in [0, 1] {
                add(&mut counters, Place::field(*l, k, Ty::Int(r.t)));
            }
        }
    }
    for x in &field_cells {
        add(&mut counters, Place::local(*x));
    }
    let mut moved: HashSet<(BasicBlock, Statement)> = HashSet::new(); // the steps already moved
    let names: Vec<BasicBlock> = bd.blocks.iter().map(|b| b.name).collect();
    for name in names {
        let mut i = 0;
        while i < bd.block(name).unwrap().statements.len() {
            let b = bd.block(name).unwrap();
            if moved.contains(&(name, b.statements[i].clone())) {
                i += 1;
                continue;
            }
            let lines = b.lines_of();
            let mut v_: Option<Local> = None;
            let mut s_: Option<Place> = None;
            let mut n: Option<(BinOp, Operand)> = None;
            let v = copy_into(&b.statements[i]).filter(|(_, src)| counters.contains(src));
            if let Some((v0, v1)) = v {
                n = step_of(lines[i + 1], v1, &Place::local(v0));
                if n.is_some() {
                    v_ = Some(v0);
                    s_ = Some(v1.clone());
                }
            }
            if n.is_none() {
                for c in &counters {
                    n = step_of(lines[i], c, c);
                    if n.is_some() {
                        s_ = Some(c.clone());
                        break;
                    }
                }
            }
            let Some(s) = s_ else {
                i += 1;
                continue;
            };
            let (op, one) = n.unwrap();
            if let Some(v) = v_ {
                if bd.sets(v) != 1 {
                    i += 1;
                    continue;
                }
            }
            let hs = bd.preds(name);
            if hs.len() != 1 {
                i += 1;
                continue;
            }
            // the loop's own back edges: blocks into the test that can only be reached through it
            let h = hs[0];
            let around = bd.reached(Some(h));
            let latches: Vec<BasicBlock> = bd.preds(h).into_iter().filter(|x| !around.contains(x)).collect();
            if latches.is_empty() {
                i += 1;
                continue;
            }
            // the counter itself is read nowhere else in the loop, but in its test
            let width = if v_.is_some() { 2 } else { 1 };
            let read_elsewhere = bd.blocks.iter().any(|x| {
                x.name != h
                    && !around.contains(&x.name)
                    && x.lines_of().iter().enumerate().any(|(j, l)| {
                        !(x.name == name && j >= i && j < i + width) && l.operands().iter().any(|o| reads(o, &s))
                    })
            });
            if read_elsewhere {
                i += 1;
                continue;
            }
            let inc = Statement::Assign(s.clone(), Rvalue::BinaryOp(op, Operand::Copy(s.clone()), one));
            let bi = bd.at(name).unwrap();
            bd.blocks[bi].statements.drain(i..i + width);
            if !loops.iter().any(|l| l.0 == h) {
                loops.push((h, name));
            }
            if let Some(v) = v_ {
                bd.replace_word(v, &s);
                bd.rename_debug(v, &s);
                retype.set(v, Ty::Unit);
            }
            // each latch steps the counter. A block that only steps it goes last, so the layout can put it straight
            // after the body, which then falls through into it.
            for l in &latches {
                let li = bd.at(*l).unwrap();
                if bd.blocks[li].terminator == Terminator::Goto(h) {
                    bd.blocks[li].statements.push(inc.clone());
                    moved.insert((*l, inc.clone()));
                    if bd.blocks[li].statements.len() == 1 && bd.preds(*l).len() == 1 {
                        let x = bd.blocks.remove(li);
                        bd.blocks.push(x);
                    }
                } else {
                    let fresh = BasicBlock(next_block);
                    bd.blocks.push(Block {
                        name: fresh,
                        statements: vec![inc.clone()],
                        terminator: Terminator::Goto(h),
                    });
                    moved.insert((fresh, inc.clone()));
                    for t in bd.blocks[li].terminator.targets_mut() {
                        if *t == h {
                            *t = fresh;
                        }
                    }
                    next_block += 1;
                }
            }
            break;
        }
    }
    // A local that is only ever a copy of a counter, read only while the counter still holds what was copied, can be
    // the counter itself (the variable `i`, and rustc's temporaries for it).
    let steady = |bd: &Body, x: Local, s: &Place, b: BasicBlock, i: usize| -> bool {
        let mut seen: HashSet<(BasicBlock, usize, bool)> = HashSet::new();
        let mut work: Vec<(BasicBlock, usize, bool)> = vec![(b, i + 1, false)];
        while let Some((name, from, dirty)) = work.pop() {
            let Some(blk) = bd.block(name) else { continue };
            if !seen.insert((name, from, dirty)) {
                continue;
            }
            let (mut d, mut stop) = (dirty, false);
            let lines = blk.lines_of();
            for (j, l) in lines.iter().enumerate().skip(from) {
                if name == b && j == i {
                    stop = true; // copied again
                    break;
                }
                if mentions(*l, x) > 0 && d {
                    return false;
                }
                if l.dest() == Some(s) {
                    d = true;
                }
            }
            if !stop {
                for t in blk.terminator.successors() {
                    work.push((t, 0, d));
                }
            }
        }
        true
    };
    let mut changed = true;
    while changed {
        changed = false;
        let all = bd.clone();
        for s in &counters {
            for bi in 0..bd.blocks.len() {
                let mut i = 0;
                while i < bd.blocks[bi].statements.len() {
                    let Statement::Assign(d, Rvalue::Use(Operand::Copy(src) | Operand::Move(src))) =
                        &bd.blocks[bi].statements[i]
                    else {
                        i += 1;
                        continue;
                    };
                    let Some(x) = d.as_local().filter(|l| *l != Local::UNIT) else {
                        i += 1;
                        continue;
                    };
                    if src != s || x == Local(0) || !decl.has(&x) || is_arg(x) {
                        i += 1;
                        continue;
                    }
                    let set_by = |l: Line| {
                        l.dest().is_some_and(|p| {
                            p.base == PlaceBase::Local(x)
                                && (p.projection.is_empty()
                                    || (matches!(p.projection[0], Projection::Field(..)) && p.wrapped() == 1))
                        })
                    };
                    let defs = all.all_lines().filter(|l| set_by(*l)).count();
                    if defs != 1 || all.taken(x) {
                        i += 1;
                        continue;
                    }
                    let name = bd.blocks[bi].name;
                    if !steady(&bd, x, s, name, i) {
                        i += 1;
                        continue;
                    }
                    bd.blocks[bi].statements.remove(i);
                    bd.replace_word(x, s);
                    bd.rename_debug(x, s);
                    retype.set(x, Ty::Unit);
                    changed = true;
                    break;
                }
            }
            if changed {
                break;
            }
        }
    }
    // Layout follows text order, and rustc's inlined loops put the body far from the test. Each loop's step block
    // goes after its test, followed by the straight run of blocks it leads into (each reached only from the one
    // before, or the test of an inner loop), as a hand-written loop's body follows its test.
    let mut step_of_loop: Ordered<BasicBlock, BasicBlock> = Ordered::new();
    for (h, st) in &loops {
        step_of_loop.set(*h, *st);
    }
    for (h, _) in &loops {
        let mut seen: HashSet<BasicBlock> = HashSet::from([*h]);
        let mut cur = bd.block(*h).map(|b| b.name);
        while let Some(c) = cur {
            let next = if let Some(st) = step_of_loop.get(&c) {
                bd.block(*st).map(|b| b.name)
            } else {
                let next = match &bd.block(c).unwrap().terminator {
                    Terminator::Goto(n) => Some(*n),
                    Terminator::Call { target: Some(n), .. } => Some(*n),
                    _ => None,
                };
                next.and_then(|n| bd.block(n).map(|b| b.name))
                    .filter(|n| bd.preds(*n).len() == 1 || step_of_loop.has(n))
            };
            let Some(next) = next else { break };
            if seen.contains(&next) || next == BasicBlock(0) {
                break;
            }
            seen.insert(next);
            let x = bd.blocks.remove(bd.at(next).unwrap());
            let at = bd.at(c).unwrap() + 1;
            bd.blocks.insert(at, x);
            cur = Some(next);
        }
    }
    // declarations: new types, and the fresh locals
    for d in bd.decls.iter_mut() {
        if let Some(t) = retype.get(&d.local) {
            d.ty = t.clone();
        }
    }
    let added: Vec<Decl> = retype
        .iter()
        .filter(|(l, _)| !decl.has(l))
        .map(|(l, t)| Decl {
            local: *l,
            mutable: true,
            ty: t.clone(),
        })
        .collect();
    bd.decls.splice(0..0, added);
    f.body = bd;
}

/// A type printed as one word.
fn one_word(t: &Ty) -> bool {
    match t {
        Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str => true,
        Ty::Path(p) => p.qself.is_none() && matches!(p.segments.as_slice(), [Segment::Name { args: None, .. }]),
        _ => false,
    }
}

/// `Rev::<core::ops::Range<T>>` (or RangeInclusive), a reversed range's type in a literal.
fn rev_of_range(p: &Path) -> bool {
    let [Segment::Name { name, args: Some(g) }] = p.segments.as_slice() else {
        return false;
    };
    if p.qself.is_some() || name != "Rev" || !g.turbofish {
        return false;
    }
    matches!(g.args.as_slice(), [GenericArg::Ty(Ty::Path(inner))] if core_range(inner).is_some())
}

/// `Option::<T>::Some`, for T a card's type: T.
fn option_some(p: &Path) -> Option<IntTy> {
    let [Segment::Name { name: o, args: Some(g) }, Segment::Name { name: s, args: None }] = p.segments.as_slice()
    else {
        return None;
    };
    if p.qself.is_some() || o != "Option" || s != "Some" || !g.turbofish {
        return None;
    }
    match g.args.as_slice() {
        [GenericArg::Ty(Ty::Int(t @ (IntTy::I16 | IntTy::U8 | IntTy::Usize)))] => Some(*t),
        _ => None,
    }
}

/// Does the operand read `s`, or an element of it (`copy s`, `move s[i]`)?
fn reads(o: &Operand, s: &Place) -> bool {
    let (Operand::Copy(p) | Operand::Move(p)) = o else {
        return false;
    };
    p.base == s.base
        && p.projection.len() >= s.projection.len()
        && p.projection[..s.projection.len()] == s.projection[..]
        && p.projection[s.projection.len()..].iter().all(|q| {
            matches!(
                q,
                Projection::Index(_)
                    | Projection::IndexPlus(..)
                    | Projection::ConstantIndex { .. }
                    | Projection::IndexPlace(_)
            )
        })
}

/// Where a place is the field `proj`, or starts with it, the local `to` takes its place.
fn field_to(p: &mut Place, proj: &Place, to: &Place) {
    for q in p.projection.iter_mut() {
        if let Projection::IndexPlace(inner) = q {
            if **inner == *proj {
                *q = Projection::Index(to.as_local().unwrap());
            } else {
                field_to(inner, proj, to);
            }
        }
    }
    if p.base == proj.base && p.projection.first() == proj.projection.first() && !proj.projection.is_empty() {
        let rest: Vec<Projection> = p.projection[1..].to_vec();
        *p = Place {
            base: to.base.clone(),
            projection: to.projection.iter().cloned().chain(rest).collect(),
        };
    }
}

/// An operand that reads the field `proj` reads `to` instead.
fn read_to(o: &mut Operand, proj: &Place, to: &Operand) {
    if matches!(o, Operand::Copy(p) | Operand::Move(p) if p == proj) {
        *o = to.clone();
    }
}

/// The first operand that moves x becomes a copy of it.
fn first_move_to_copy(ops: Vec<&mut Operand>, x: Local) {
    let moved = Place::local(x);
    if let Some(o) = ops.into_iter().find(|o| matches!(o, Operand::Move(p) if *p == moved)) {
        *o = Operand::Copy(moved);
    }
}
