//! Arrays of arrays. A `[[T; W]; H]` of scalars is one array: by tens, each row 10 cells after the one before, where
//! the program is compiled that way (`stride`), else row after row, as Rust lays it out. `t[r][c]` is `t[r * S + c]`
//! for the stride S, its index worked out on the line before the one that names it, after rustc's checks of r and c.
//! A reference to one is a reference to the flat array, and the rows it is built of (`[move _2; 3]`, `_2` a row built
//! just before) are its elements written out. Anything else done with a row (borrowed, copied out, iterated) leaves
//! the program's arrays of arrays as they are, and refused. By tens, the cells between the rows are holes, which
//! nothing reads or writes, and `holes::fill` puts the program's other cells there.
use std::collections::{HashMap, HashSet};

use super::ir::{
    predecessors, ArrayLen, BasicBlock, BinOp, Block, BorrowKind, Const, Decl, IntTy, Local, Locals, Operand, Place,
    PlaceBase, Projection, Rvalue, Statement, Terminator, Ty,
};
use super::structs::Function;
use super::util::Ordered;

/// The grids laid out by tens, by function and local: each one's width and stride.
pub type Holes = HashMap<(String, Local), (u64, u64)>;

/// An array of arrays of scalars, or a reference to one: its width W, its height H, and its stride S, how far apart
/// its rows start (10 by tens, else W).
#[derive(Clone, Copy)]
struct Grid {
    w: u64,
    h: u64,
    s: u64,
    by_ref: bool,
}

impl Grid {
    /// The cells it takes as one array: every row S apart, and the last row only as long as it is.
    fn len(&self) -> u64 {
        (self.h - 1) * self.s + self.w
    }

    /// The cells between one row's end and the next row's start, which nothing reads.
    fn padded(&self) -> bool {
        self.s != self.w
    }
}

/// The stride of a grid W wide and H high: by tens, 10, where `tens` and its rows fit in ten cells (and it has more
/// than one, of more than one cell: a column is a list, its index already the row); else W, row after row.
fn stride(w: u64, h: u64, tens: bool) -> u64 {
    if tens && (2..10).contains(&w) && h > 1 {
        10
    } else {
        w
    }
}

/// A `[[T; W]; H]` of scalars (or a reference or a pointer to one): the grid, and the type it becomes, laid out by
/// tens where `tens` is.
fn grid(t: &Ty, tens: bool) -> Option<(Grid, Ty)> {
    let flat = |t: &Ty| {
        let Ty::Array(row, ArrayLen::Known(h)) = t else {
            return None;
        };
        let Ty::Array(e, ArrayLen::Known(w)) = &**row else {
            return None;
        };
        let s = stride(*w, *h, tens);
        (matches!(**e, Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool) && *w > 0 && *h > 0)
            .then(|| ((*w, *h, s), Ty::Array(e.clone(), ArrayLen::Known((h - 1) * s + w))))
    };
    match t {
        Ty::Ref { lifetime, mutable, to } => flat(to).map(|((w, h, s), t)| {
            let to = Box::new(t);
            (
                Grid { w, h, s, by_ref: true },
                Ty::Ref {
                    lifetime: lifetime.clone(),
                    mutable: *mutable,
                    to,
                },
            )
        }),
        Ty::Ptr { mutable, to } => flat(to).map(|((w, h, s), t)| {
            (
                Grid { w, h, s, by_ref: true },
                Ty::Ptr {
                    mutable: *mutable,
                    to: Box::new(t),
                },
            )
        }),
        t => flat(t).map(|((w, h, s), t)| (Grid { w, h, s, by_ref: false }, t)),
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
fn grids(fns: &Ordered<String, Function>, tens: bool) -> Option<HashMap<String, HashMap<Local, Grid>>> {
    let mut all = HashMap::new();
    for (name, f) in fns.iter() {
        let typed = f
            .params
            .iter()
            .map(|(l, t)| (*l, t))
            .chain(f.body.decls.iter().map(|d| (d.local, &d.ty)));
        let gs: HashMap<Local, Grid> = typed
            .chain([(Local(0), &f.ret)])
            .filter_map(|(l, t)| grid(t, tens).map(|(g, _)| (l, g)))
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
                            matched &= grid(t, tens).is_some();
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
/// values nothing changes in between: the lines that build the rows, and the grid's elements. A row may be named
/// more than once, as GVN leaves two rows of zeros. By tens, the cells between one row and the next are written as
/// well, with a zero (or, where the row has no constant to tell its type, a copy of its last value); nothing reads
/// them.
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
    let mut built_rows: Vec<Vec<Operand>> = vec![];
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
        if d.as_local() != Some(row) {
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
        if !lines.contains(&at) {
            lines.push(at);
        }
        built_rows.push(
            values
                .into_iter()
                .map(|v| match v {
                    Operand::Move(p) => Operand::Copy(p),
                    v => v,
                })
                .collect(),
        );
    }
    let built_rows: Vec<Vec<Operand>> = match repeat {
        Some(h) => vec![built_rows[0].clone(); h as usize],
        None => built_rows,
    };
    let first = &built_rows[0][0];
    let rv = if matches!(first, Operand::Constant(_)) && built_rows.iter().flatten().all(|e| e == first) {
        Rvalue::Repeat(first.clone(), g.len())
    } else {
        let mut elements = vec![];
        for (r, row) in built_rows.iter().enumerate() {
            elements.extend(row.iter().cloned());
            if r + 1 < built_rows.len() {
                let gap = row.iter().find_map(zero).unwrap_or_else(|| row[row.len() - 1].clone());
                elements.extend(std::iter::repeat_n(gap, (g.s - g.w) as usize));
            }
        }
        Rvalue::Array(elements)
    };
    Some((lines, rv))
}

/// A zero of a constant's type, for the cells between rows.
fn zero(o: &Operand) -> Option<Operand> {
    match o {
        Operand::Constant(Const::Int { ty, .. }) => Some(Operand::Constant(Const::Int { value: 0, ty: *ty })),
        Operand::Constant(Const::Bool(_)) => Some(Operand::Constant(Const::Bool(false))),
        _ => None,
    }
}

/// Make every array of arrays of scalars one array, in every function: or none, where one of them is used in a way
/// the pass doesn't write. With `tens`, a grid less than ten wide is laid out by tens, each row ten cells after the
/// one before. The grids laid out by tens, function by function, each with its width and stride: the cells between
/// its rows are holes, which nothing in the program reads or writes.
pub fn arrays_of_arrays(fns: &mut Ordered<String, Function>, tens: bool) -> Holes {
    let Some(all) = grids(fns, tens) else {
        return Holes::new();
    };
    let mut padded = Holes::new();
    for (name, gs) in &all {
        for (l, g) in gs.iter().filter(|(_, g)| g.padded() && !g.by_ref) {
            padded.insert((name.clone(), *l), (g.w, g.s));
        }
    }
    for (name, gs) in &all {
        let f = fns.get_mut(name).unwrap();
        for (l, t) in f.params.iter_mut() {
            if gs.contains_key(l) {
                *t = grid(t, tens).unwrap().1;
            }
        }
        if gs.contains_key(&Local(0)) {
            f.ret = grid(&f.ret, tens).unwrap().1;
        }
        for d in f.body.decls.iter_mut() {
            if gs.contains_key(&d.local) {
                d.ty = grid(&d.ty, tens).unwrap().1;
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
        let taken: HashSet<Local> = f
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .filter_map(|s| match s {
                Statement::Assign(_, Rvalue::Ref(_, p)) => p.base_local(),
                _ => None,
            })
            .collect();
        let params: HashSet<Local> = f.params.iter().map(|(l, _)| *l).collect();
        let mut ix = Indexes {
            next,
            locals: HashMap::new(),
            fold: folded(&f.body.blocks, gs, &taken, &params),
            taken,
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
    padded
}

/// The indexes of the grids that are another local plus a constant, `_33 = copy _7` or `_33 = Add(copy _7,
/// const 1_usize)` (or the checked add's value): in `t[r + 1][c + 2]`, r and c, and 1 and 2. A grid's element is
/// then found from r * S + c, worked out once for every neighbour, and an offset of S + 2 (`folded`). Such an index
/// v is set once, and wherever a grid reads it, the way back to its line (in the block, then each block with one
/// way in) sets neither it nor the local it is made of. A plain add is folded only where that local is never a
/// usize past a card (`small`): `r + 1` can wrap to 0 from usize::MAX, which r * S can't stand for. A checked add
/// can't wrap, and once `r + 1 < H` holds, r is below H too.
fn folded(
    blocks: &[Block],
    gs: &HashMap<Local, Grid>,
    taken: &HashSet<Local>,
    params: &HashSet<Local>,
) -> HashMap<Local, (Local, u64)> {
    let preds = predecessors(blocks);
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let mut defs: HashMap<Local, usize> = HashMap::new();
    for b in blocks {
        for s in &b.statements {
            if let Statement::Assign(d, _) = s {
                *defs.entry(d.base_local().unwrap()).or_default() += 1;
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            *defs.entry(dest.base_local().unwrap()).or_default() += 1;
        }
    }
    let small = small_usizes(blocks, taken, params);
    let k_of = |o: &Operand| match o {
        Operand::Constant(Const::Int {
            value,
            ty: IntTy::Usize,
        }) if (0..=99).contains(value) => Some(*value as u64),
        _ => None,
    };
    let whole = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    };
    // the lines before line `j` of block `bi` (the line itself is `statements.len()` for the terminator), back to
    // where `v` is set: that line, or none where a way in is not the only one, or `x` is set first
    let walk = |bi: usize, j: usize, v: Local, guard: &[Local]| -> Option<&Rvalue> {
        let (mut bi, mut j) = (bi, j);
        for _ in 0..16 {
            for s in blocks[bi].statements[..j].iter().rev() {
                if let Statement::Assign(d, rv) = s {
                    let l = d.base_local();
                    if l == Some(v) && d.as_local() == Some(v) {
                        return Some(rv);
                    }
                    if l.is_some_and(|l| guard.contains(&l) || l == v) {
                        return None;
                    }
                }
            }
            let ins = preds.get(&blocks[bi].name).map(Vec::as_slice).unwrap_or_default();
            let ([p], true) = (ins, bi != 0) else { return None };
            let &pj = at.get(p)?;
            if let Terminator::Call { dest, .. } = &blocks[pj].terminator {
                if dest.base_local().is_some_and(|l| guard.contains(&l) || l == v) {
                    return None;
                }
            }
            bi = pj;
            j = blocks[pj].statements.len();
        }
        None
    };
    let mut out: HashMap<Local, Option<(Local, u64)>> = HashMap::new();
    for (bi, b) in blocks.iter().enumerate() {
        let lines = b.statements.len();
        let mut uses: Vec<(usize, Local)> = vec![];
        for (j, s) in b.statements.iter().enumerate() {
            for p in s.places() {
                uses.extend(grid_indexes(&p, gs).into_iter().map(|v| (j, v)));
            }
        }
        for p in b.terminator.places() {
            uses.extend(grid_indexes(&p, gs).into_iter().map(|v| (lines, v)));
        }
        for (j, v) in uses {
            let once = defs.get(&v) == Some(&1) && !taken.contains(&v);
            let found = once
                .then(|| walk(bi, j, v, &[]))
                .flatten()
                .and_then(|rv| match rv {
                    Rvalue::Use(o) => whole(o).map(|x| (x, 0)),
                    Rvalue::BinaryOp(BinOp::Add, x, k) => whole(x).filter(|x| small.contains(x)).zip(k_of(k)),
                    _ => None,
                })
                // (or the checked add's value, `move (_34.0: usize)`, where `_34 = AddWithOverflow(copy _7, const 1)`)
                .or_else(|| {
                    once.then(|| checked_add(blocks, bi, j, v, &walk, &whole, &k_of))
                        .flatten()
                })
                .filter(|(x, _)| *x != v && !taken.contains(x) && walk_keeps(&walk, bi, j, v, *x));
            let entry = out.entry(v).or_insert(found);
            if *entry != found {
                *entry = None;
            }
        }
    }
    out.into_iter().filter_map(|(v, f)| f.map(|f| (v, f))).collect()
}

/// Whether x keeps its value from the line that sets v to line j of block bi: the walk back finds v's line with x
/// guarded.
fn walk_keeps<'a>(
    walk: &dyn Fn(usize, usize, Local, &[Local]) -> Option<&'a Rvalue>,
    bi: usize,
    j: usize,
    v: Local,
    x: Local,
) -> bool {
    walk(bi, j, v, &[x]).is_some()
}

/// v set as a checked add's value, `v = move (_34.0: usize)` after `_34 = AddWithOverflow(copy x, const k)`: x and k.
fn checked_add<'a>(
    blocks: &'a [Block],
    bi: usize,
    j: usize,
    v: Local,
    walk: &dyn Fn(usize, usize, Local, &[Local]) -> Option<&'a Rvalue>,
    whole: &dyn Fn(&Operand) -> Option<Local>,
    k_of: &dyn Fn(&Operand) -> Option<u64>,
) -> Option<(Local, u64)> {
    let Rvalue::Use(Operand::Copy(t) | Operand::Move(t)) = walk(bi, j, v, &[])? else {
        return None;
    };
    let ([Projection::Field(0, _)], Some(pair)) = (t.projection.as_slice(), t.base_local()) else {
        return None;
    };
    // the pair's line, before v's: found by walking back from v's own line
    let (vb, vj) = line_of(blocks, bi, j, v)?;
    let Rvalue::BinaryOp(BinOp::AddWithOverflow, x, k) = walk(vb, vj, pair, &[])? else {
        return None;
    };
    let x = whole(x)?;
    walk(vb, vj, pair, &[x])?;
    Some((x, k_of(k)?))
}

/// Where v is set, walking back from line j of block bi as `folded` does: its block and line.
fn line_of(blocks: &[Block], bi: usize, j: usize, v: Local) -> Option<(usize, usize)> {
    let preds = predecessors(blocks);
    let at: HashMap<BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let (mut bi, mut j) = (bi, j);
    for _ in 0..16 {
        if let Some(k) = blocks[bi].statements[..j]
            .iter()
            .rposition(|s| matches!(s, Statement::Assign(d, _) if d.as_local() == Some(v)))
        {
            return Some((bi, k));
        }
        let ins = preds.get(&blocks[bi].name).map(Vec::as_slice).unwrap_or_default();
        let ([p], true) = (ins, bi != 0) else { return None };
        bi = *at.get(p)?;
        j = blocks[bi].statements.len();
    }
    None
}

/// The locals a grid reads as its row or column index at a place.
fn grid_indexes(p: &Place, gs: &HashMap<Local, Grid>) -> Vec<Local> {
    let Some(g) = p.base_local().and_then(|l| gs.get(&l)) else {
        return vec![];
    };
    match two_indexes(p, *g) {
        Some((r, c)) => [r, c]
            .into_iter()
            .filter_map(|x| match x {
                Ix::Var(v) => Some(v),
                Ix::At(_) => None,
            })
            .collect(),
        None => vec![],
    }
}

/// The usize locals that are never past a card: not a parameter nor taken, and set only from constants, copies of
/// such locals, and their sums with a constant (`c = c + 1`, `r = _33`), checked or not. A cast to usize can be
/// past a card (`-1i16 as usize`), and so can a difference or a call's result; so they, and what is made from them,
/// are not.
fn small_usizes(blocks: &[Block], taken: &HashSet<Local>, params: &HashSet<Local>) -> HashSet<Local> {
    let mut sets: HashMap<Local, Vec<&Rvalue>> = HashMap::new();
    let mut not: HashSet<Local> = taken.iter().chain(params).copied().collect();
    for b in blocks {
        for s in &b.statements {
            if let Statement::Assign(d, rv) = s {
                match d.as_local() {
                    Some(l) => sets.entry(l).or_default().push(rv),
                    None => {
                        not.extend(d.base_local());
                    }
                }
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            not.extend(dest.base_local());
        }
    }
    let mut small: HashSet<Local> = sets.keys().filter(|l| !not.contains(l)).copied().collect();
    loop {
        let from = |o: &Operand, small: &HashSet<Local>| match o {
            Operand::Constant(Const::Int { value, .. }) => (0..=999).contains(value),
            Operand::Copy(p) | Operand::Move(p) => match (p.base_local(), p.projection.as_slice()) {
                (Some(l), []) => small.contains(&l),
                // a checked add's value, `(_34.0: usize)`: small when its pair is
                (Some(l), [Projection::Field(0, _)]) => small.contains(&l),
                _ => false,
            },
            _ => false,
        };
        let keep: HashSet<Local> = small
            .iter()
            .copied()
            .filter(|l| {
                sets[l].iter().all(|rv| match rv {
                    Rvalue::Use(o) => from(o, &small),
                    Rvalue::BinaryOp(BinOp::Add | BinOp::AddWithOverflow, x, y) => from(x, &small) && from(y, &small),
                    _ => false,
                })
            })
            .collect();
        if keep.len() == small.len() {
            return small;
        }
        small = keep;
    }
}

/// A part of a flat index worked out into a local of its own: r * S, or r * S + c.
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
    /// the indexes that are another local plus a constant where every grid reads them (see `folded`)
    fold: HashMap<Local, (Local, u64)>,
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

    /// `t[r][c]` as `t[r * S + c]`: a constant index where both are, an index plus a constant where the row is one
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
        // each index as a local plus a constant, the constants together one offset from the element they name
        let fold = |x: Ix| match x {
            Ix::Var(v) => match self.fold.get(&v) {
                Some(&(y, k)) if k == 0 || !g.by_ref => (Ix::Var(y), k),
                _ => (x, 0),
            },
            x => (x, 0),
        };
        let ((r, kr), (c, kc)) = (fold(r), fold(c));
        let off = kr * g.s + kc;
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
                offset: r * g.s + c,
                min_length: g.len(),
            },
            (Ix::At(0), Ix::Var(c)) => Projection::Index(c),
            (Ix::At(r), Ix::Var(c)) if !g.by_ref => Projection::IndexPlus(c, (r * g.s) as u32),
            (Ix::Var(r), c) => {
                let row = match g.s {
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
                        Projection::Index(self.part(Part::Sum(Ix::Var(r), c, g.s), sum, avail, &mut before))
                    }
                }
            }
            (r @ Ix::At(k), c) => {
                let sum = Rvalue::BinaryOp(BinOp::Add, usize(k * g.s), operand(c));
                Projection::Index(self.part(Part::Sum(r, c, g.s), sum, avail, &mut before))
            }
        };
        let at = match (at, off) {
            (at, 0) => at,
            (Projection::Index(l), off) => Projection::IndexPlus(l, off as u32),
            (Projection::IndexPlus(l, k), off) => Projection::IndexPlus(l, k + off as u32),
            (Projection::ConstantIndex { offset, min_length }, off) => Projection::ConstantIndex {
                offset: offset + off,
                min_length,
            },
            (at, _) => at,
        };
        let deref = g.by_ref.then_some(Projection::Deref);
        *p = Place {
            base: PlaceBase::Local(p.base_local().unwrap()),
            projection: deref.into_iter().chain([at]).collect(),
        };
    }
}
