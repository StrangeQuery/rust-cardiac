//! u16 and u32 where the standard library's methods work in them: on an i16, `abs_diff` and `unsigned_abs` return
//! a u16 and `ilog10` a u32, and at opt-level 2 rustc inlines `abs_diff` as a difference of two `as u16` casts and
//! a u8's `midpoint` as a sum in u16. A card holds neither type, so such a local is a cell only where the program
//! can't tell. It is an i16 that holds either
//!   its value, where that is known to be on a card (0..=999): a constant, a cast of a u8 or a bool, a sum,
//!     difference or quotient of those whose bounds stay on the card, or a method's result that does
//!     (`methods::exact_result`). It can then be used as any number on a card is;
//!   or else a number on a card equal to its value modulo 2^16 (2^32 for a u32): `-500i16 as u16` is 65036, held
//!     as -500. A cast to i16 or u8 keeps that true (65036 as i16 is -500), and so does a sum or a difference that
//!     is exact, so each is overflow-checked here, in every profile: past a card it panics, as an i16 does.
//!     Nothing else may read such a local: not a comparison, a division, a wider cast or a call.
//! A local that is neither keeps its type, which the lowering refuses.
use std::collections::HashMap;

use super::front::local_types;
use super::ir::visit_mut::{walk_const, walk_ty, VisitMut};
use super::ir::{
    occurrences, BinOp, Body, CastKind, Const, File, IntTy, Item, Local, Locals, Operand, Place, Projection, Rvalue,
    Statement, Terminator, Ty, Unwind,
};
use super::methods::Gains;
use super::runtime::NO_CARD;

/// What a u16 or u32 local's cell holds (see the top of the file), as far as is known yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    /// its value, between these bounds, on a card
    Exact(i128, i128),
    /// its value modulo the type's size
    Modular,
    /// neither: it keeps its type
    Refused,
}

impl Held {
    fn join(self, other: Held) -> Held {
        match (self, other) {
            (Held::Exact(a, b), Held::Exact(c, d)) => Held::Exact(a.min(c), b.max(d)),
            (Held::Refused, _) | (_, Held::Refused) => Held::Refused,
            _ => Held::Modular,
        }
    }

    /// Bounds on the card are exact; past it the value is known only modulo the type's size.
    fn within(lo: i128, hi: i128) -> Held {
        if lo >= 0 && hi <= 999 {
            Held::Exact(lo, hi)
        } else {
            Held::Modular
        }
    }
}

/// What a use of a local needs it to hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Needs {
    Either,
    Exact,
}

fn is_wide(t: &Ty) -> bool {
    matches!(t, Ty::Int(IntTy::U16 | IntTy::U32))
}

fn width(t: IntTy) -> u32 {
    match t {
        IntTy::I8 | IntTy::U8 => 8,
        IntTy::I16 | IntTy::U16 => 16,
        IntTy::I32 | IntTy::U32 => 32,
        IntTy::I128 | IntTy::U128 => 128,
        _ => 64,
    }
}

/// Every function's u16 and u32 locals that can be cells are i16s.
pub fn wide_locals(file: &mut File) {
    for item in &mut file.items {
        if let Item::Fn(f) = item {
            wide_in(&f.params, &mut f.body);
        }
    }
}

/// The whole local an operand reads.
fn read(o: &Operand) -> Option<Local> {
    match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    }
}

fn wide_in(params: &[(Local, Ty)], body: &mut Body) {
    let types = local_types(params, &body.decls);
    let (cells, held) = wide_cells(body, params, &types);
    if cells.is_empty() {
        return;
    }
    rewrite(params, body, &cells, &types, &held);
}

/// The u16 and u32 locals that can be cells, and what each holds.
fn wide_cells(body: &Body, params: &[(Local, Ty)], types: &HashMap<Local, Ty>) -> (Vec<Local>, HashMap<Local, Held>) {
    // not a parameter or the result, which a caller passes or reads as the type
    let wide: Vec<Local> = body
        .decls
        .iter()
        .filter(|d| is_wide(&d.ty) && d.local != Local(0) && !params.iter().any(|(l, _)| *l == d.local))
        .map(|d| d.local)
        .collect();
    if wide.is_empty() {
        return (vec![], HashMap::new());
    }
    let mut held: HashMap<Local, Held> = HashMap::new();
    // a sum in a loop could widen its bounds a step a round: after a few rounds, a value still growing is modular
    let mut rounds = 0;
    loop {
        rounds += 1;
        let before = held.clone();
        let mut join = |x: Local, h: Held| {
            let g = match held.get(&x) {
                Some(&Held::Exact(lo, hi)) if rounds > 8 && h.join(Held::Exact(lo, hi)) != Held::Exact(lo, hi) => {
                    Held::Modular.join(h)
                }
                Some(g) => g.join(h),
                None => h,
            };
            held.insert(x, g);
        };
        // what each definition gives
        for b in &body.blocks {
            for s in &b.statements {
                if let Statement::Assign(d, rv) = s {
                    if let Some(x) = d.as_local().filter(|x| wide.contains(x)) {
                        if let Some(h) = value(rv, &types, &before) {
                            join(x, h);
                        }
                    }
                }
            }
            if let Terminator::Call { dest, func, .. } = &b.terminator {
                if let Some(x) = dest.as_local().filter(|x| wide.contains(x)) {
                    join(
                        x,
                        super::methods::exact_result(func).map_or(Held::Refused, |[lo, hi]| Held::Exact(lo, hi)),
                    );
                }
            }
        }
        // what each use needs; a use of any other kind and it keeps its type
        for &w in &wide {
            let (seen, needs) = uses(body, w, &types);
            let h = held.get(&w).copied();
            let refused = seen != occurrences(&body.blocks, w) || (needs == Needs::Exact && h == Some(Held::Modular));
            if refused {
                held.insert(w, Held::Refused);
            }
        }
        if held == before {
            break;
        }
    }
    // (one never set from what is known keeps its type too)
    let cells: Vec<Local> = wide
        .into_iter()
        .filter(|w| held.get(w).is_some_and(|h| *h != Held::Refused))
        .collect();
    (cells, held)
}

/// What a definition `x = rv` of a u16 or u32 gives it; none while an operand isn't known yet.
fn value(rv: &Rvalue, types: &HashMap<Local, Ty>, held: &HashMap<Local, Held>) -> Option<Held> {
    let of = |o: &Operand| -> Option<Held> {
        match o {
            Operand::Constant(Const::Int { value, .. }) if (0..=999).contains(value) => {
                Some(Held::Exact(*value, *value))
            }
            _ => match read(o) {
                Some(w) if is_wide(types.get(&w)?) => held.get(&w).copied(),
                _ => Some(Held::Refused),
            },
        }
    };
    Some(match rv {
        Rvalue::Use(o) => of(o)?,
        Rvalue::Cast(o, to, super::ir::CastKind::IntToInt) => {
            let Some(from) = read(o).and_then(|l| types.get(&l)) else {
                return Some(Held::Refused);
            };
            match from {
                Ty::Bool => Held::Exact(0, 1),
                Ty::Int(IntTy::U8) => Held::Exact(0, 255),
                Ty::Int(IntTy::I16 | IntTy::I32 | IntTy::Usize) => Held::Modular,
                // a wider cast of a value known only modulo the narrower type's size: refused
                Ty::Int(t @ (IntTy::U16 | IntTy::U32)) => match (of(o)?, to) {
                    (Held::Modular, Ty::Int(to)) if width(*to) > width(*t) => Held::Refused,
                    (h, _) => h,
                },
                _ => Held::Refused,
            }
        }
        Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub), a, b) => match (of(a)?, of(b)?) {
            (Held::Refused, _) | (_, Held::Refused) => Held::Refused,
            (Held::Exact(p, q), Held::Exact(r, s)) if *op == BinOp::Add => Held::within(p + r, q + s),
            (Held::Exact(p, q), Held::Exact(r, s)) => Held::within(p - s, q - r),
            _ => Held::Modular,
        },
        Rvalue::BinaryOp(op @ (BinOp::Div | BinOp::Rem), a, Operand::Constant(Const::Int { value: k, .. }))
            if *k >= 1 =>
        {
            match of(a)? {
                Held::Exact(lo, hi) if *op == BinOp::Div => Held::Exact(lo / k, hi / k),
                Held::Exact(_, hi) => Held::Exact(0, hi.min(k - 1)),
                _ => Held::Refused,
            }
        }
        _ => Held::Refused,
    })
}

/// How many times the function names `w` in a way this file knows, and what the strictest of those needs it to hold.
fn uses(body: &Body, w: Local, types: &HashMap<Local, Ty>) -> (usize, Needs) {
    let mut seen = 0;
    let mut needs = Needs::Either;
    let mut need = |n: Needs| {
        seen += 1;
        if n == Needs::Exact {
            needs = Needs::Exact;
        }
    };
    let is = |o: &Operand| read(o) == Some(w);
    for b in &body.blocks {
        for s in &b.statements {
            match s {
                Statement::StorageLive(x) | Statement::StorageDead(x) if *x == w => need(Needs::Either),
                Statement::Assign(d, rv) => {
                    let into = d.as_local().filter(|x| types.get(x).is_some_and(is_wide));
                    if d.as_local() == Some(w) {
                        need(Needs::Either);
                    }
                    match rv {
                        // a cast to i16 or u8, or into another of these, is right of a value known modulo its size,
                        // and so is a sum or difference into another; any other cast needs the value
                        Rvalue::Cast(o, Ty::Int(to), super::ir::CastKind::IntToInt) if is(o) => {
                            let narrow = matches!(to, IntTy::I16 | IntTy::U8)
                                || (into.is_some() && width(*to) <= width(types_int(types, w)));
                            need(if narrow { Needs::Either } else { Needs::Exact });
                        }
                        Rvalue::Use(o) if is(o) && into.is_some() => need(Needs::Either),
                        Rvalue::BinaryOp(BinOp::Add | BinOp::Sub, a, c) if into.is_some() => {
                            for o in [a, c] {
                                if is(o) {
                                    need(Needs::Either);
                                }
                            }
                        }
                        Rvalue::BinaryOp(BinOp::Div | BinOp::Rem, a, Operand::Constant(_))
                            if is(a) && into.is_some() =>
                        {
                            need(Needs::Exact)
                        }
                        Rvalue::BinaryOp(
                            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge,
                            a,
                            c,
                        ) => {
                            for o in [a, c] {
                                if is(o) {
                                    need(Needs::Exact);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        match &b.terminator {
            Terminator::SwitchInt { discr, .. } if is(discr) => need(Needs::Exact),
            Terminator::Call { dest, .. } if dest.as_local() == Some(w) => need(Needs::Either),
            _ => {}
        }
    }
    (seen, needs)
}

fn types_int(types: &HashMap<Local, Ty>, l: Local) -> IntTy {
    match types.get(&l) {
        Some(Ty::Int(t)) => *t,
        _ => IntTy::I16,
    }
}

/// The cells' locals are i16s, and so are the constants they meet; each sum or difference into one that can leave
/// the card is overflow-checked, its block split at it.
fn rewrite(
    params: &[(Local, Ty)],
    body: &mut Body,
    cells: &[Local],
    types: &HashMap<Local, Ty>,
    held: &HashMap<Local, Held>,
) {
    let i16 = Ty::Int(IntTy::I16);
    let narrow = |o: &mut Operand| {
        if let Operand::Constant(Const::Int {
            ty: ty @ (IntTy::U16 | IntTy::U32),
            ..
        }) = o
        {
            *ty = IntTy::I16;
        }
    };
    let mut checked = vec![];
    for (bi, b) in body.blocks.iter_mut().enumerate() {
        for (si, s) in b.statements.iter_mut().enumerate() {
            let Statement::Assign(d, rv) = s else { continue };
            let into = d.as_local().is_some_and(|x| cells.contains(&x));
            let reads = rv
                .operands()
                .iter()
                .any(|o| read(o).is_some_and(|l| cells.contains(&l)));
            if !into && !reads {
                continue;
            }
            let sum = matches!(rv, Rvalue::BinaryOp(BinOp::Add | BinOp::Sub, ..));
            if into && sum && value(rv, types, held) == Some(Held::Modular) {
                checked.push((bi, si));
            }
            match rv {
                Rvalue::Cast(_, to, _) if into => *to = i16.clone(),
                Rvalue::Use(o) => narrow(o),
                Rvalue::BinaryOp(_, a, c) => {
                    narrow(a);
                    narrow(c);
                }
                _ => {}
            }
        }
    }
    for d in body.decls.iter_mut().filter(|d| cells.contains(&d.local)) {
        d.ty = i16.clone();
    }
    // last first, so that each split leaves the places of those before it as they were
    for (bi, si) in checked.into_iter().rev() {
        check(params, body, bi, si);
    }
}

/// Statement `si` of block `bi`, `w = Add(a, b)` or `Sub`, as rustc writes an overflow-checked one: the op with its
/// flag, an assert of the flag, and w set from the pair in a block of its own, where the rest of the block goes.
fn check(params: &[(Local, Ty)], body: &mut Body, bi: usize, si: usize) {
    let Statement::Assign(w, Rvalue::BinaryOp(op, a, b)) = body.blocks[bi].statements[si].clone() else {
        unreachable!("a sum or a difference")
    };
    let copy = |o: Operand| match o {
        Operand::Move(p) => Operand::Copy(p),
        o => o,
    };
    let (a, b) = (copy(a), copy(b));
    let (checked, sign) = if op == BinOp::Add {
        (BinOp::AddWithOverflow, "+")
    } else {
        (BinOp::SubWithOverflow, "-")
    };
    let mut gains = Gains::new(params, &body.blocks, &mut body.decls);
    let pair = gains.local(Ty::Tuple(vec![Ty::Int(IntTy::I16), Ty::Bool]));
    let field = |k: u32, t: Ty| Place {
        base: Place::local(pair).base,
        projection: vec![Projection::Field(k, t)],
    };
    let block = &mut body.blocks[bi];
    let mut rest = vec![Statement::Assign(
        w,
        Rvalue::Use(Operand::Move(field(0, Ty::Int(IntTy::I16)))),
    )];
    rest.extend(block.statements.split_off(si + 1));
    block.statements[si] = Statement::Assign(Place::local(pair), Rvalue::BinaryOp(checked, a.clone(), b.clone()));
    let end = std::mem::replace(&mut block.terminator, Terminator::Unreachable);
    let rest = gains.block(rest, end);
    let gained = std::mem::take(&mut gains.blocks);
    body.blocks[bi].terminator = Terminator::Assert {
        cond: Operand::Move(field(1, Ty::Bool)),
        expected: false,
        msg: format!("attempt to compute `{{}} {sign} {{}}`, which would overflow"),
        msg_args: vec![a, b],
        target: rest,
        unwind: Unwind::Continue,
    };
    body.blocks.extend(gained);
}

/// The program's own u16 locals that `wide_locals` can't make cells, where they are compared, multiplied or
/// divided, say: each is an i16 that holds its value, as an i16 does, and is never below 0, as a u16 isn't. So what
/// a cast from a signed type or a usize sets it to is checked to be on a card, and so is what a difference does,
/// in every profile: below 0, Rust's value is at least 65536 - 999. With overflow checks on, a difference below 0
/// panics as Rust's does; without them it panics as an i16 past a card does. Sums, products and quotients of
/// values not below 0 stay so, and compare and cast as Rust's do. Its uses: those, comparisons, `&`, `<<` and `>>`
/// by a constant, a switch, the pair an overflow-checked op makes, and an assert's message. A group of locals
/// that one names another in is made cells all together, or none of it is: where one has any other use, where one
/// is a local of a function rustc inlined (core's may rely on a u16's wrapping), or where `wide_locals` can make
/// every one a cell, which is exact where Rust's value is past a card and its own cast back is on one.
pub fn u16_cells(file: &mut File) {
    for item in &mut file.items {
        if let Item::Fn(f) = item {
            u16_cells_in(&f.params, &mut f.body);
        }
    }
}

const U16: Ty = Ty::Int(IntTy::U16);

/// The u16 pair an overflow-checked op on two u16s makes.
fn is_u16_pair(t: &Ty) -> bool {
    matches!(t, Ty::Tuple(ts) if ts.as_slice() == [U16, Ty::Bool])
}

/// What setting a cell needs checked first: a cast's operand, or a difference's two sides, overflow-checked or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Check {
    Cast(IntTy),
    Difference { checked: bool },
}

fn u16_cells_in(params: &[(Local, Ty)], body: &mut Body) {
    let types = local_types(params, &body.decls);
    let inlined: Vec<Local> = body.inlined.iter().map(|(l, _)| *l).collect();
    let mut cells: Vec<Local> = body
        .decls
        .iter()
        .filter(|d| (d.ty == U16 || is_u16_pair(&d.ty)) && d.local != Local(0) && !inlined.contains(&d.local))
        .filter(|d| !params.iter().any(|(l, _)| *l == d.local))
        .map(|d| d.local)
        .collect();
    // a cell only where every line that names it is one known here, with the cells as they are
    loop {
        let mut named: HashMap<Local, usize> = HashMap::new();
        for b in &body.blocks {
            let known = b
                .statements
                .iter()
                .map(|s| known_statement(s, &cells, &types).is_some().then_some(s as &dyn Locals))
                .chain([known_terminator(&b.terminator, &cells).then_some(&b.terminator as &dyn Locals)]);
            for line in known.flatten() {
                for &c in &cells {
                    *named.entry(c).or_default() += line.count(c);
                }
            }
        }
        let before = cells.len();
        cells.retain(|c| named.get(c).copied().unwrap_or(0) == occurrences(&body.blocks, *c));
        if cells.len() == before {
            break;
        }
    }
    // the groups: each cell with those it is named with
    let mut group: HashMap<Local, usize> = cells.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    loop {
        let mut joined = false;
        for line in body.blocks.iter().flat_map(|b| b.lines()) {
            let named: Vec<usize> = cells.iter().filter(|&&c| line.mentions(c)).map(|c| group[c]).collect();
            if let Some(&least) = named.iter().min() {
                for c in &cells {
                    if named.contains(&group[c]) && group[c] != least {
                        group.insert(*c, least);
                        joined = true;
                    }
                }
            }
        }
        if !joined {
            break;
        }
    }
    let (wide, _) = wide_cells(body, params, &types);
    let ours: Vec<usize> = cells.iter().filter(|c| !wide.contains(c)).map(|c| group[c]).collect();
    cells.retain(|c| ours.contains(&group[c]));
    if cells.is_empty() {
        return;
    }
    let mut checks = vec![];
    struct Narrow;
    impl VisitMut for Narrow {
        fn visit_ty(&mut self, t: &mut Ty) {
            if *t == U16 {
                *t = Ty::Int(IntTy::I16);
            }
            walk_ty(self, t)
        }
        fn visit_const(&mut self, c: &mut Const) {
            if let Const::Int {
                ty: ty @ IntTy::U16, ..
            } = c
            {
                *ty = IntTy::I16;
            }
            walk_const(self, c)
        }
    }
    let names_a_cell = |line: &dyn Locals| cells.iter().any(|&c| line.mentions(c));
    for (bi, b) in body.blocks.iter_mut().enumerate() {
        for (si, s) in b.statements.iter_mut().enumerate() {
            if !names_a_cell(s) {
                continue;
            }
            if let Some(Some(check)) = known_statement(s, &cells, &types) {
                checks.push((bi, si, check));
            }
            Narrow.visit_statement(s);
        }
        if names_a_cell(&b.terminator) {
            Narrow.visit_terminator(&mut b.terminator);
        }
    }
    for d in body.decls.iter_mut().filter(|d| cells.contains(&d.local)) {
        Narrow.visit_ty(&mut d.ty);
    }
    // last first, so that each split leaves the places of those before it as they were
    for (bi, si, check) in checks.into_iter().rev() {
        check_first(params, body, bi, si, check);
    }
}

/// Whether a u16 operand is a cell or a constant a card holds, not below 0.
fn u16_operand(o: &Operand, cells: &[Local]) -> bool {
    match o {
        Operand::Constant(Const::Int { value, ty: IntTy::U16 }) => (0..=999).contains(value),
        _ => read(o).is_some_and(|l| cells.contains(&l)),
    }
}

/// Whether a statement is one the cells may be named in, and if so what it needs checked first.
fn known_statement(s: &Statement, cells: &[Local], types: &HashMap<Local, Ty>) -> Option<Option<Check>> {
    let ok = |o: &Operand| u16_operand(o, cells);
    let cell = |l: Option<Local>| l.is_some_and(|l| cells.contains(&l));
    // `(_8.0: u16)` or `(_8.1: bool)` of a pair
    let of_pair = |o: &Operand, k: u32| match o {
        Operand::Copy(p) | Operand::Move(p) => {
            cell(p.base_local()) && matches!(p.projection.as_slice(), [Projection::Field(j, _)] if *j == k)
        }
        _ => false,
    };
    let (dest, rv) = match s {
        Statement::StorageLive(_) | Statement::StorageDead(_) => return Some(None),
        Statement::Assign(d, rv) => (d, rv),
        _ => return None,
    };
    let into = dest.as_local().filter(|l| cells.contains(l));
    let into_ty = into.and_then(|l| types.get(&l));
    let comparison = |op: &BinOp| {
        matches!(
            op,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Cmp
        )
    };
    match (into_ty, rv) {
        (Some(t), Rvalue::Use(o)) if *t == U16 && (ok(o) || of_pair(o, 0)) => Some(None),
        (Some(t), Rvalue::Cast(o, to, CastKind::IntToInt)) if *t == U16 && *to == U16 => {
            let from = match o {
                Operand::Constant(Const::Int { value, .. }) => return (0..=999).contains(value).then_some(None),
                _ => types.get(&read(o)?)?,
            };
            match from {
                Ty::Bool | Ty::Int(IntTy::U8) => Some(None),
                _ if ok(o) => Some(None),
                Ty::Int(t @ (IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::Isize | IntTy::Usize)) => {
                    Some(Some(Check::Cast(*t)))
                }
                _ => None,
            }
        }
        (Some(t), Rvalue::BinaryOp(op, a, b)) if *t == U16 => match op {
            BinOp::Add | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::AddUnchecked | BinOp::MulUnchecked
                if ok(a) && ok(b) =>
            {
                Some(None)
            }
            BinOp::Sub | BinOp::SubUnchecked if ok(a) && ok(b) => Some(Some(Check::Difference { checked: false })),
            BinOp::BitAnd | BinOp::Shl | BinOp::Shr if ok(a) && matches!(b, Operand::Constant(_)) => Some(None),
            _ => None,
        },
        (Some(t), Rvalue::BinaryOp(op, a, b)) if is_u16_pair(t) && ok(a) && ok(b) => match op {
            BinOp::AddWithOverflow | BinOp::MulWithOverflow => Some(None),
            BinOp::SubWithOverflow => Some(Some(Check::Difference { checked: true })),
            _ => None,
        },
        (Some(_), _) => None,
        // what reads a cell into something else
        (None, Rvalue::Cast(o, Ty::Int(to), CastKind::IntToInt)) if ok(o) && *to != IntTy::U16 => Some(None),
        (None, Rvalue::BinaryOp(op, a, b)) if comparison(op) && ok(a) && ok(b) => Some(None),
        (None, Rvalue::Use(o)) if of_pair(o, 1) => Some(None),
        _ => None,
    }
}

/// Whether a terminator is one the cells may be named in: an assert of a pair's flag (its message may name cells),
/// or a switch on a cell whose arms are on a card.
fn known_terminator(t: &Terminator, cells: &[Local]) -> bool {
    let ok = |o: &Operand| u16_operand(o, cells) || !cells.iter().any(|&c| o.mentions(c));
    match t {
        Terminator::Assert { cond, msg_args, .. } => {
            let flag = match cond {
                Operand::Copy(p) | Operand::Move(p) => {
                    !p.base_local().is_some_and(|l| cells.contains(&l))
                        || matches!(p.projection.as_slice(), [Projection::Field(1, _)])
                }
                _ => true,
            };
            flag && msg_args.iter().all(ok)
        }
        Terminator::SwitchInt { discr, arms, .. } => {
            u16_operand(discr, cells) && arms.iter().all(|(v, _)| (0..=999).contains(v))
        }
        _ => false,
    }
}

/// The check `check` before statement `si` of block `bi`: the block ends there in an assert, and the statement and
/// the rest of the block go on in a block of their own.
fn check_first(params: &[(Local, Ty)], body: &mut Body, bi: usize, si: usize, check: Check) {
    let Statement::Assign(_, rv) = &body.blocks[bi].statements[si] else {
        unreachable!("a cell set")
    };
    let copy = |o: &Operand| match o {
        Operand::Move(p) => Operand::Copy(p.clone()),
        o => o.clone(),
    };
    let (test, msg, msg_args) = match (check, rv) {
        // a usize past a card is held below 0, above every usize a card holds
        (Check::Cast(IntTy::Usize), Rvalue::Cast(o, ..)) => (
            Rvalue::BinaryOp(
                BinOp::Le,
                copy(o),
                Operand::Constant(Const::Int {
                    value: 999,
                    ty: IntTy::Usize,
                }),
            ),
            NO_CARD.to_string(),
            vec![],
        ),
        (Check::Cast(ty), Rvalue::Cast(o, ..)) => (
            Rvalue::BinaryOp(BinOp::Ge, copy(o), Operand::Constant(Const::Int { value: 0, ty })),
            NO_CARD.to_string(),
            vec![],
        ),
        (Check::Difference { checked }, Rvalue::BinaryOp(_, a, b)) => (
            Rvalue::BinaryOp(BinOp::Ge, copy(a), copy(b)),
            if checked {
                "attempt to compute `{} - {}`, which would overflow".to_string()
            } else {
                NO_CARD.to_string()
            },
            if checked { vec![copy(a), copy(b)] } else { vec![] },
        ),
        _ => unreachable!("a check of what was found"),
    };
    let mut gains = Gains::new(params, &body.blocks, &mut body.decls);
    let flag = gains.local(Ty::Bool);
    let block = &mut body.blocks[bi];
    let rest = block.statements.split_off(si);
    block.statements.push(Statement::Assign(Place::local(flag), test));
    let end = std::mem::replace(&mut block.terminator, Terminator::Unreachable);
    let rest = gains.block(rest, end);
    let gained = std::mem::take(&mut gains.blocks);
    body.blocks[bi].terminator = Terminator::Assert {
        cond: Operand::Move(Place::local(flag)),
        expected: true,
        msg,
        msg_args,
        target: rest,
        unwind: Unwind::Continue,
    };
    body.blocks.extend(gained);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir;

    fn body(lines: &str) -> Body {
        ir::parse_body(&format!(
            "let mut _0: ();\n let _1: i16;\n let _2: i16;\n let mut _3: u16;\n let mut _4: u16;\n let mut _5: u16;\n let mut _6: i16;\n let mut _7: bool;\n bb0: {{\n {lines}\n return;\n }}\n"
        ))
    }

    fn ty(b: &Body, l: u32) -> Ty {
        b.decls.iter().find(|d| d.local == Local(l)).unwrap().ty.clone()
    }

    #[test]
    fn a_difference_of_casts_read_as_an_i16_is_checked() {
        let mut b = body("_3 = copy _2 as u16 (IntToInt);\n _4 = copy _1 as u16 (IntToInt);\n _5 = Sub(copy _3, copy _4);\n _6 = copy _5 as i16 (IntToInt);");
        wide_in(&[], &mut b);
        assert!([3, 4, 5].iter().all(|&l| ty(&b, l) == Ty::Int(IntTy::I16)));
        assert!(matches!(
            b.blocks[0].statements[2],
            Statement::Assign(_, Rvalue::BinaryOp(BinOp::SubWithOverflow, ..))
        ));
        assert!(matches!(b.blocks[0].terminator, Terminator::Assert { .. }));
    }

    #[test]
    fn a_value_known_only_modulo_its_size_is_not_compared() {
        let mut b = body("_3 = copy _2 as u16 (IntToInt);\n _7 = Lt(copy _3, const 5_u16);");
        wide_in(&[], &mut b);
        assert_eq!(ty(&b, 3), Ty::Int(IntTy::U16));
    }

    #[test]
    fn a_sum_of_two_u8s_halved_is_exact() {
        let mut b = ir::parse_body(
            "let mut _0: ();\n let _1: u8;\n let _2: u8;\n let mut _3: u16;\n let mut _4: u16;\n let mut _5: u16;\n let mut _6: u8;\n bb0: {\n _3 = copy _1 as u16 (IntToInt);\n _4 = copy _2 as u16 (IntToInt);\n _5 = Add(move _3, move _4);\n _3 = Div(move _5, const 2_u16);\n _6 = move _3 as u8 (IntToInt);\n return;\n }\n",
        );
        wide_in(&[], &mut b);
        assert!([3, 4, 5].iter().all(|&l| ty(&b, l) == Ty::Int(IntTy::I16)));
        // nothing to check: at most 510
        assert!(matches!(b.blocks[0].terminator, Terminator::Return));
    }

    #[test]
    fn a_compared_cast_of_an_i16_is_checked_to_be_on_a_card() {
        let mut b = body("_3 = copy _2 as u16 (IntToInt);\n _4 = copy _1 as u16 (IntToInt);\n _7 = Lt(copy _3, copy _4);\n _5 = Sub(copy _3, copy _4);\n _6 = copy _5 as i16 (IntToInt);");
        u16_cells_in(&[], &mut b);
        assert!([3, 4, 5].iter().all(|&l| ty(&b, l) == Ty::Int(IntTy::I16)));
        // each cast, and the difference, after a test that ends its block
        let mut tests: Vec<String> = b
            .blocks
            .iter()
            .filter_map(|b| match &b.terminator {
                Terminator::Assert { .. } => b.statements.last().map(|s| s.to_string()),
                _ => None,
            })
            .collect();
        tests.sort();
        assert_eq!(
            tests,
            [
                "_10 = Ge(copy _2, const 0_i16);",
                "_8 = Ge(copy _3, copy _4);",
                "_9 = Ge(copy _1, const 0_i16);"
            ]
        );
    }

    #[test]
    fn a_group_the_modular_cells_take_whole_is_theirs() {
        let mut b = body("_3 = copy _2 as u16 (IntToInt);\n _4 = copy _1 as u16 (IntToInt);\n _5 = Sub(copy _3, copy _4);\n _6 = copy _5 as i16 (IntToInt);");
        u16_cells_in(&[], &mut b);
        assert_eq!(ty(&b, 3), Ty::Int(IntTy::U16));
        assert_eq!(b.blocks.len(), 1);
    }
}
