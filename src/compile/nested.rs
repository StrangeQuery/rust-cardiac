//! Arrays of arrays. A `[[T; W]; H]` of scalars is one array of H * W elements, row after row, as Rust lays it out:
//! `t[r][c]` is `t[r * W + c]`, its index worked out on the line before the one that names it, after rustc's checks
//! of r and c. A reference to one is a reference to the flat array, and the rows it is built of (`[move _2; 3]`,
//! `_2` a row built just before) are its elements written out. Anything else done with a row (borrowed, copied out,
//! iterated) leaves the program's arrays of arrays as they are, and refused.
use std::collections::{HashMap, HashSet};

use super::ir::{
    predecessors, ArrayLen, BasicBlock, BinOp, Block, BorrowKind, Const, Decl, IntTy, Local, Locals, Operand, Place,
    PlaceBase, Projection, Rvalue, Statement, Terminator, Ty,
};
use super::structs::Function;
use super::util::Ordered;

/// An array of arrays of scalars, or a reference to one: its width W and height H.
#[derive(Clone, Copy)]
struct Grid {
    w: u64,
    h: u64,
    by_ref: bool,
}

/// A `[[T; W]; H]` of scalars (or a reference or a pointer to one): the grid, and the type it becomes.
fn grid(t: &Ty) -> Option<(Grid, Ty)> {
    let flat = |t: &Ty| {
        let Ty::Array(row, ArrayLen::Known(h)) = t else {
            return None;
        };
        let Ty::Array(e, ArrayLen::Known(w)) = &**row else {
            return None;
        };
        matches!(**e, Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool)
            .then(|| ((*w, *h), Ty::Array(e.clone(), ArrayLen::Known(w * h))))
    };
    match t {
        Ty::Ref { lifetime, mutable, to } => flat(to).map(|((w, h), t)| {
            let to = Box::new(t);
            (
                Grid { w, h, by_ref: true },
                Ty::Ref {
                    lifetime: lifetime.clone(),
                    mutable: *mutable,
                    to,
                },
            )
        }),
        Ty::Ptr { mutable, to } => flat(to).map(|((w, h), t)| {
            (
                Grid { w, h, by_ref: true },
                Ty::Ptr {
                    mutable: *mutable,
                    to: Box::new(t),
                },
            )
        }),
        t => flat(t).map(|((w, h), t)| (Grid { w, h, by_ref: false }, t)),
    }
}

/// A row or column index: a local, or a constant.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Ix {
    Var(Local),
    At(u64),
}

/// `t[r][c]` (or `(*t)[r][c]` through a reference): its two indexes.
fn two_indexes(p: &Place, g: Grid) -> Option<(Ix, Ix)> {
    let ix = |q: &Projection| match q {
        Projection::Index(l) => Some(Ix::Var(*l)),
        Projection::ConstantIndex { offset, .. } => Some(Ix::At(*offset)),
        _ => None,
    };
    match (g.by_ref, p.projection.as_slice()) {
        (false, [r, c]) | (true, [Projection::Deref, r, c]) => Some((ix(r)?, ix(c)?)),
        _ => None,
    }
}

/// The program's grids, function by function, if every use of every one is one this pass writes: else none.
fn grids(fns: &Ordered<String, Function>) -> Option<HashMap<String, HashMap<Local, Grid>>> {
    let mut all = HashMap::new();
    for (name, f) in fns.iter() {
        let typed = f
            .params
            .iter()
            .map(|(l, t)| (*l, t))
            .chain(f.body.decls.iter().map(|d| (d.local, &d.ty)));
        let gs: HashMap<Local, Grid> = typed
            .chain([(Local(0), &f.ret)])
            .filter_map(|(l, t)| grid(t).map(|(g, _)| (l, g)))
            .collect();
        if !gs.is_empty() {
            all.insert(name.clone(), gs);
        }
    }
    if all.is_empty() {
        return None;
    }
    for (name, gs) in &all {
        let f = fns.get(name).unwrap();
        let whole = |p: &Place, by_ref: bool| {
            p.as_local()
                .and_then(|l| gs.get(&l))
                .is_some_and(|g| g.by_ref == by_ref)
        };
        let indexed = |p: &Place| {
            p.base_local()
                .and_then(|l| gs.get(&l))
                .is_some_and(|g| two_indexes(p, *g).is_some())
        };
        let named = |x: &dyn Locals| gs.keys().map(|l| x.count(*l)).sum::<usize>();
        for b in &f.body.blocks {
            for (i, s) in b.statements.iter().enumerate() {
                let fine = match s {
                    Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop => true,
                    // a whole grid copied, borrowed (or reborrowed), or built of its rows
                    Statement::Assign(d, rv) if whole(d, false) || whole(d, true) => match rv {
                        Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => whole(p, whole(d, true)),
                        Rvalue::Ref(BorrowKind::Shared | BorrowKind::Mut, p) => {
                            whole(d, true)
                                && (whole(p, false)
                                    || p.projection == [Projection::Deref]
                                        && p.base_local().is_some_and(|l| whole(&Place::local(l), true)))
                        }
                        Rvalue::Repeat(..) | Rvalue::Array(_) => {
                            !whole(d, true)
                                && rows(&f.body.blocks, &b.statements, i, gs[&d.as_local().unwrap()]).is_some()
                        }
                        _ => false,
                    },
                    s => s.places().iter().filter(|p| indexed(p)).count() == named(s),
                };
                if !fine {
                    return None;
                }
            }
            let t = &b.terminator;
            let fine = match t {
                // a whole grid passed to the program's own function, or returned by one
                Terminator::Call { dest, func, args, .. } if fns.has(&func.to_string()) => {
                    // (each whole grid where the callee's own parameter or result is one too)
                    let callee = fns.get(&func.to_string()).unwrap();
                    let theirs = callee.params.iter().map(|(_, t)| t).chain([&callee.ret]);
                    let ours = args.iter().map(Operand::place).chain([Some(dest)]);
                    let mut count = 0;
                    let mut matched = true;
                    for (p, t) in ours.zip(theirs) {
                        let Some(p) = p else { continue };
                        if whole(p, false) || whole(p, true) {
                            matched &= grid(t).is_some();
                            count += 1;
                        } else if indexed(p) {
                            count += 1;
                        }
                    }
                    matched && args.len() == callee.params.len() && count == named(t)
                }
                t => t.places().iter().filter(|p| indexed(p)).count() == named(t),
            };
            if !fine {
                return None;
            }
        }
    }
    Some(all)
}

/// A grid built of its rows, `_1 = [move _2; 3]` or `_1 = [move _2, move _3]` (line i of a block), each row a local
/// built earlier in the block (`_2 = [const 0_i16; 3]`, `_2 = [copy _5, copy _6]`) and named nowhere else, whose
/// values nothing changes in between: the lines that build the rows, and the grid's elements.
fn rows(blocks: &[Block], statements: &[Statement], i: usize, g: Grid) -> Option<(Vec<usize>, Rvalue)> {
    let Statement::Assign(_, rv) = &statements[i] else {
        return None;
    };
    let (ops, repeat) = match rv {
        Rvalue::Repeat(o, n) if *n == g.h => (vec![o], Some(g.h)),
        Rvalue::Array(os) if os.len() as u64 == g.h => (os.iter().collect(), None),
        _ => return None,
    };
    let mut lines = vec![];
    let mut elements: Vec<Operand> = vec![];
    for o in ops {
        let row = o.local()?;
        let storage = |s: &Statement| matches!(s, Statement::StorageLive(_) | Statement::StorageDead(_));
        let naming = blocks
            .iter()
            .flat_map(|b| &b.statements)
            .filter(|s| !storage(s) && s.mentions(row))
            .count();
        if naming != 2 || blocks.iter().any(|b| b.terminator.mentions(row)) {
            return None;
        }
        let at = (0..i).rev().find(|k| statements[*k].mentions(row))?;
        let Statement::Assign(d, built) = &statements[at] else {
            return None;
        };
        if d.as_local() != Some(row) || lines.contains(&at) {
            return None;
        }
        let values = match built {
            Rvalue::Repeat(v, n) if *n == g.w => vec![v.clone(); g.w as usize],
            Rvalue::Array(vs) if vs.len() as u64 == g.w => vs.clone(),
            _ => return None,
        };
        // each value a constant or a whole local, which no line between sets
        for v in &values {
            match v {
                Operand::Constant(_) => {}
                Operand::Copy(p) | Operand::Move(p) if p.projection.is_empty() => {
                    let l = p.base_local()?;
                    if statements[at + 1..i]
                        .iter()
                        .any(|s| matches!(s, Statement::Assign(d, _) if d.base_local() == Some(l)))
                    {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        lines.push(at);
        elements.extend(values.into_iter().map(|v| match v {
            Operand::Move(p) => Operand::Copy(p),
            v => v,
        }));
    }
    let rv = match repeat {
        Some(_)
            if matches!(elements.first(), Some(Operand::Constant(_))) && elements.iter().all(|e| *e == elements[0]) =>
        {
            Rvalue::Repeat(elements[0].clone(), g.w * g.h)
        }
        Some(_) => Rvalue::Array(elements.iter().cloned().cycle().take((g.w * g.h) as usize).collect()),
        None => Rvalue::Array(elements),
    };
    Some((lines, rv))
}

/// Make every array of arrays of scalars one array, in every function: or none, where one of them is used in a way
/// the pass doesn't write.
pub fn arrays_of_arrays(fns: &mut Ordered<String, Function>) {
    let Some(all) = grids(fns) else { return };
    for (name, gs) in &all {
        let f = fns.get_mut(name).unwrap();
        for (l, t) in f.params.iter_mut() {
            if gs.contains_key(l) {
                *t = grid(t).unwrap().1;
            }
        }
        if gs.contains_key(&Local(0)) {
            f.ret = grid(&f.ret).unwrap().1;
        }
        for d in f.body.decls.iter_mut() {
            if gs.contains_key(&d.local) {
                d.ty = grid(&d.ty).unwrap().1;
            }
        }
        let next = f
            .params
            .iter()
            .map(|(l, _)| l.0)
            .chain(f.body.decls.iter().map(|d| d.local.0))
            .max()
            .unwrap_or(0)
            + 1;
        // a grid built of its rows: its elements, and the rows' lines gone
        for bi in 0..f.body.blocks.len() {
            for i in 0..f.body.blocks[bi].statements.len() {
                let Statement::Assign(d, Rvalue::Repeat(..) | Rvalue::Array(_)) = &f.body.blocks[bi].statements[i]
                else {
                    continue;
                };
                let Some(g) = d.as_local().and_then(|l| gs.get(&l)) else {
                    continue;
                };
                let (lines, rv) = rows(&f.body.blocks, &f.body.blocks[bi].statements, i, *g).unwrap();
                let b = &mut f.body.blocks[bi];
                let Statement::Assign(_, r) = &mut b.statements[i] else {
                    unreachable!()
                };
                *r = rv;
                for k in lines {
                    b.statements[k] = Statement::Nop;
                }
            }
        }
        // each `t[r][c]`: its flat index, worked out on the lines before, unless every way there has worked it out
        // already, with neither index changed since
        let mut ix = Indexes {
            next,
            locals: HashMap::new(),
            taken: f
                .body
                .blocks
                .iter()
                .flat_map(|b| &b.statements)
                .filter_map(|s| match s {
                    Statement::Assign(_, Rvalue::Ref(_, p)) => p.base_local(),
                    _ => None,
                })
                .collect(),
        };
        let held = ix.held_on_entry(&f.body.blocks, gs);
        for (b, mut avail) in f.body.blocks.iter_mut().zip(held) {
            let mut statements = vec![];
            for mut s in std::mem::take(&mut b.statements) {
                s.places_mut(&mut |p| ix.flatten(p, gs, &mut avail, Some(&mut statements)));
                if let Statement::Assign(d, _) = &s {
                    kill(&mut avail, d);
                }
                statements.push(s);
            }
            b.terminator
                .places_mut(&mut |p| ix.flatten(p, gs, &mut avail, Some(&mut statements)));
            b.statements = statements;
        }
        let mut fresh: Vec<Local> = ix.locals.into_values().collect();
        fresh.sort();
        f.body.decls.extend(fresh.into_iter().map(|local| Decl {
            local,
            mutable: false,
            ty: Ty::Int(IntTy::Usize),
        }));
    }
}

/// A part of a flat index worked out into a local of its own: r * W, or r * W + c.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Part {
    Row(Local, u64),
    Sum(Ix, Ix, u64),
}

impl Part {
    fn reads(&self, v: Local) -> bool {
        match self {
            Part::Row(r, _) => *r == v,
            Part::Sum(r, c, _) => [r, c].contains(&&Ix::Var(v)),
        }
    }
}

/// What a line that sets a local (or a part of one) leaves of the parts held.
fn kill(avail: &mut HashSet<Part>, d: &Place) {
    if let Some(v) = d.base_local() {
        avail.retain(|e| !e.reads(v));
    }
}

/// The locals the parts of flat indexes are worked out into, one for each part, so that a part worked out on every
/// way into a line is in the same local there.
struct Indexes {
    next: u32,
    locals: HashMap<Part, Local>,
    /// the locals whose address is taken, which a write through it may change: their parts are never held
    taken: HashSet<Local>,
}

impl Indexes {
    /// The parts held on the way into each block: those every way into it has worked out (none into the first).
    fn held_on_entry(&mut self, blocks: &[Block], gs: &HashMap<Local, Grid>) -> Vec<HashSet<Part>> {
        let preds = predecessors(blocks);
        let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
        let mut out: Vec<Option<HashSet<Part>>> = vec![None; blocks.len()];
        let entry = |i: usize, out: &[Option<HashSet<Part>>]| -> HashSet<Part> {
            let ins = preds.get(&blocks[i].name).map(Vec::as_slice).unwrap_or_default();
            if i == 0 || ins.is_empty() {
                return HashSet::new();
            }
            let mut held: Option<HashSet<Part>> = None;
            for p in ins {
                if let Some(o) = &out[at[p]] {
                    held = Some(match held {
                        None => o.clone(),
                        Some(h) => h.intersection(o).copied().collect(),
                    });
                }
            }
            held.unwrap_or_default()
        };
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..blocks.len() {
                let mut avail = entry(i, &out);
                for s in &blocks[i].statements {
                    let mut s = s.clone();
                    s.places_mut(&mut |p| self.flatten(&mut p.clone(), gs, &mut avail, None));
                    if let Statement::Assign(d, _) = &s {
                        kill(&mut avail, d);
                    }
                }
                let mut t = blocks[i].terminator.clone();
                t.places_mut(&mut |p| self.flatten(&mut p.clone(), gs, &mut avail, None));
                if let Terminator::Call { dest, .. } = &t {
                    kill(&mut avail, dest);
                }
                if out[i].as_ref() != Some(&avail) {
                    out[i] = Some(avail);
                    changed = true;
                }
            }
        }
        (0..blocks.len()).map(|i| entry(i, &out)).collect()
    }

    /// A part's local: worked out by `rv` onto `before`, unless it is held.
    fn part(
        &mut self,
        e: Part,
        rv: Rvalue,
        avail: &mut HashSet<Part>,
        before: &mut Option<&mut Vec<Statement>>,
    ) -> Local {
        let next = &mut self.next;
        let l = *self.locals.entry(e).or_insert_with(|| {
            *next += 1;
            Local(*next - 1)
        });
        if !avail.contains(&e) {
            if let Some(before) = before {
                before.push(Statement::Assign(Place::local(l), rv));
            }
            if !self.taken.iter().any(|v| e.reads(*v)) {
                avail.insert(e);
            }
        }
        l
    }

    /// `t[r][c]` as `t[r * W + c]`: a constant index where both are, an index plus a constant where the row is one
    /// (on the grid itself, not through a reference), else an index whose parts it works out onto `before` (with
    /// none, only what it holds after).
    fn flatten(
        &mut self,
        p: &mut Place,
        gs: &HashMap<Local, Grid>,
        avail: &mut HashSet<Part>,
        mut before: Option<&mut Vec<Statement>>,
    ) {
        let Some(g) = p.base_local().and_then(|l| gs.get(&l)).copied() else {
            return;
        };
        let Some((r, c)) = two_indexes(p, g) else { return };
        let usize = |v: u64| {
            Operand::Constant(Const::Int {
                value: v as i128,
                ty: IntTy::Usize,
            })
        };
        let copy = |l: Local| Operand::Copy(Place::local(l));
        let operand = |x: Ix| match x {
            Ix::Var(l) => copy(l),
            Ix::At(k) => usize(k),
        };
        let at = match (r, c) {
            (Ix::At(r), Ix::At(c)) => Projection::ConstantIndex {
                offset: r * g.w + c,
                min_length: g.w * g.h,
            },
            (Ix::At(0), Ix::Var(c)) => Projection::Index(c),
            (Ix::At(r), Ix::Var(c)) if !g.by_ref => Projection::IndexPlus(c, (r * g.w) as u32),
            (Ix::Var(r), c) => {
                let row = match g.w {
                    1 => r,
                    w => self.part(
                        Part::Row(r, w),
                        Rvalue::BinaryOp(BinOp::Mul, copy(r), usize(w)),
                        avail,
                        &mut before,
                    ),
                };
                match c {
                    Ix::At(0) => Projection::Index(row),
                    Ix::At(c) if !g.by_ref => Projection::IndexPlus(row, c as u32),
                    c => {
                        let sum = Rvalue::BinaryOp(BinOp::Add, copy(row), operand(c));
                        Projection::Index(self.part(Part::Sum(Ix::Var(r), c, g.w), sum, avail, &mut before))
                    }
                }
            }
            (r @ Ix::At(k), c) => {
                let sum = Rvalue::BinaryOp(BinOp::Add, usize(k * g.w), operand(c));
                Projection::Index(self.part(Part::Sum(r, c, g.w), sum, avail, &mut before))
            }
        };
        let deref = g.by_ref.then_some(Projection::Deref);
        *p = Place {
            base: PlaceBase::Local(p.base_local().unwrap()),
            projection: deref.into_iter().chain([at]).collect(),
        };
    }
}
