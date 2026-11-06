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
use super::ir::{
    occurrences, BinOp, Body, Const, File, IntTy, Item, Local, Operand, Place, Projection, Rvalue, Statement,
    Terminator, Ty, Unwind,
};
use super::methods::Gains;

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
    // not a parameter or the result, which a caller passes or reads as the type
    let wide: Vec<Local> = body
        .decls
        .iter()
        .filter(|d| is_wide(&d.ty) && d.local != Local(0) && !params.iter().any(|(l, _)| *l == d.local))
        .map(|d| d.local)
        .collect();
    if wide.is_empty() {
        return;
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
    if cells.is_empty() {
        return;
    }
    rewrite(params, body, &cells, &types, &held);
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
}
