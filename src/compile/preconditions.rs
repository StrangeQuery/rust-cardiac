//! The standard library's checks of unsafe preconditions, which rustc keeps with debug assertions on (core's
//! `ub_checks`): `x.unchecked_add(y)` first calls `unchecked_add::precondition_check(x, y)`, a function of core's
//! whose MIR no crate carries, which panics without unwinding if the sum overflows. Each is an assert of what it
//! checks, with Rust's message, so a violation panics as any failed check does, and the range pass drops one it
//! proves (a `for` loop's step).
//!
//! ```text
//!     _17 = core::num::<impl usize>::unchecked_add::precondition_check(copy _14, const 1_usize) -> [return: bb12, ..];
//! ```
//!
//! is
//!
//! ```text
//!     _18 = AddWithOverflow(copy _14, const 1_usize);
//!     assert(!move (_18.1: bool), "unsafe precondition(s) violated: usize::unchecked_add cannot overflow") -> bb12;
//! ```
use std::collections::HashMap;

use super::ir::{
    BasicBlock, BinOp, Block, Const, Decl, File, IntTy, Item, Local, Operand, Path, Place, PlaceBase, Projection,
    Rvalue, Segment, Statement, Terminator, Ty, Unwind,
};

/// How Rust's message for a violated precondition starts.
pub const VIOLATED: &str = "unsafe precondition(s) violated: ";

/// What a precondition check tests, by the function it is the check of.
enum Check {
    /// The op doesn't overflow `T` (unchecked_add, _sub, _mul): the overflow-checked op's flag is false.
    Overflow(BinOp, IntTy),
    /// `0 - x` doesn't overflow (unchecked_neg: x is not `T::MIN`).
    Neg(IntTy),
    /// The shift is less than `T`'s bits (unchecked_shl, _shr).
    Shift(IntTy),
    /// The index is below the length (get_unchecked, get_unchecked_mut).
    Index,
    /// The condition holds (assert_unchecked).
    Holds,
    /// Never: reaching it is the violation (unreachable_unchecked).
    Never,
}

/// The check `func` is, and Rust's message when it fails. With `debug_assertions` on, a call of
/// `core::hint::assert_unchecked` itself is its check too.
fn check(func: &Path, debug_assertions: bool) -> Option<(Check, String)> {
    let names: Vec<Option<&str>> = func
        .segments
        .iter()
        .map(|s| match s {
            Segment::Name { name, args: None } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let violated = |what: &str| format!("{VIOLATED}{what}");
    // core::hint::assert_unchecked where rustc hasn't inlined it (opt-level 1 and s): its body, which no crate's MIR
    // carries, checks the condition first when debug assertions are on, as an inlined one's precondition_check does;
    // with them off it is the program's promise. rustc prints it as plain `assert_unchecked`, the path trimmed
    if debug_assertions
        && func.qself.is_none()
        && matches!(
            names.as_slice(),
            [Some("assert_unchecked")] | [Some("core" | "std"), Some("hint"), Some("assert_unchecked")]
        )
    {
        return Some((
            Check::Holds,
            violated("hint::assert_unchecked must never be called when the condition is false"),
        ));
    }
    let [.., owner, Segment::Name { name: last, args: None }] = func.segments.as_slice() else {
        return None;
    };
    if last != "precondition_check" {
        return None;
    }
    // core::num::<impl T>::METHOD::precondition_check
    if let (None, [Some("core" | "std"), Some("num"), None, Some(method), _]) = (&func.qself, names.as_slice()) {
        let Segment::Impl(Ty::Int(t)) = &func.segments[2] else {
            return None;
        };
        let c = match *method {
            "unchecked_add" => Check::Overflow(BinOp::AddWithOverflow, *t),
            "unchecked_sub" => Check::Overflow(BinOp::SubWithOverflow, *t),
            "unchecked_mul" => Check::Overflow(BinOp::MulWithOverflow, *t),
            "unchecked_neg" => Check::Neg(*t),
            "unchecked_shl" | "unchecked_shr" => Check::Shift(*t),
            _ => return None,
        };
        return Some((c, violated(&format!("{t}::{method} cannot overflow"))));
    }
    let Segment::Name {
        name: owner,
        args: None,
    } = owner
    else {
        return None;
    };
    match (func.qself.as_ref().map(|q| &q.ty), names.len(), owner.as_str()) {
        // <usize as SliceIndex<[T]>>::get_unchecked::precondition_check
        (Some(Ty::Int(IntTy::Usize)), 2, "get_unchecked" | "get_unchecked_mut") => Some((
            Check::Index,
            violated(&format!("slice::{owner} requires that the index is within the slice")),
        )),
        (None, 2, "assert_unchecked") => Some((
            Check::Holds,
            violated("hint::assert_unchecked must never be called when the condition is false"),
        )),
        (None, 2, "unreachable_unchecked") => Some((
            Check::Never,
            violated("hint::unreachable_unchecked must never be reached"),
        )),
        _ => None,
    }
}

/// Each precondition check in the file's functions, an assert of what it checks; or none, where the test that
/// leads to it proves it holds. `debug_assertions`: whether rustc built the file with them on.
pub fn precondition_checks(file: &mut File, debug_assertions: bool) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        proved(&mut f.body.blocks);
        let mut fresh = f
            .params
            .iter()
            .map(|(l, _)| l.0)
            .chain(f.body.decls.iter().map(|d| d.local.0))
            .chain(
                f.body
                    .blocks
                    .iter()
                    .flat_map(|b| b.each_line().flat_map(|l| l.locals()))
                    .map(|l| l.0),
            )
            .max()
            .unwrap_or(0)
            + 1;
        let mut decls = vec![];
        for b in &mut f.body.blocks {
            let Terminator::Call {
                func,
                args,
                target: Some(target),
                unwind: Unwind::Continue | Unwind::Unreachable,
                ..
            } = &b.terminator
            else {
                continue;
            };
            let Some((c, msg)) = check(func, debug_assertions) else {
                continue;
            };
            let target = *target;
            // a local for what the check computes
            let mut local = |ty: Ty| {
                let l = Local(fresh);
                fresh += 1;
                decls.push(Decl {
                    local: l,
                    mutable: false,
                    ty,
                });
                l
            };
            let flag = |l: Local| {
                Operand::Move(Place {
                    base: PlaceBase::Local(l),
                    projection: vec![Projection::Field(1, Ty::Bool)],
                })
            };
            let (cond, expected) = match (c, args.as_slice()) {
                (Check::Overflow(op, t), [x, y]) => {
                    let l = local(Ty::Tuple(vec![Ty::Int(t), Ty::Bool]));
                    b.statements.push(Statement::Assign(
                        Place::local(l),
                        Rvalue::BinaryOp(op, x.clone(), y.clone()),
                    ));
                    (flag(l), false)
                }
                (Check::Neg(t), [x]) => {
                    let l = local(Ty::Tuple(vec![Ty::Int(t), Ty::Bool]));
                    let zero = Operand::Constant(Const::Int { value: 0, ty: t });
                    b.statements.push(Statement::Assign(
                        Place::local(l),
                        Rvalue::BinaryOp(BinOp::SubWithOverflow, zero, x.clone()),
                    ));
                    (flag(l), false)
                }
                (Check::Shift(t), [s]) => {
                    let Some(ty) = shift_ty(s, &f.params, &f.body.decls) else {
                        continue;
                    };
                    let l = local(Ty::Bool);
                    let bits = Operand::Constant(Const::Int { value: bits(t), ty });
                    b.statements.push(Statement::Assign(
                        Place::local(l),
                        Rvalue::BinaryOp(BinOp::Lt, s.clone(), bits),
                    ));
                    (Operand::Move(Place::local(l)), true)
                }
                (Check::Index, [i, n]) => {
                    let l = local(Ty::Bool);
                    b.statements.push(Statement::Assign(
                        Place::local(l),
                        Rvalue::BinaryOp(BinOp::Lt, i.clone(), n.clone()),
                    ));
                    (Operand::Move(Place::local(l)), true)
                }
                (Check::Holds, [c]) => (c.clone(), true),
                (Check::Never, []) => (Operand::Constant(Const::Bool(false)), true),
                _ => continue,
            };
            b.terminator = Terminator::Assert {
                cond,
                expected,
                msg,
                msg_args: vec![],
                target,
                unwind: Unwind::Continue,
            };
        }
        f.body.decls.extend(decls);
        // the cold path into a check that never holds (unreachable_unchecked's) is a way into it like any other
        let never: Vec<_> = f
            .body
            .blocks
            .iter()
            .filter(|b| matches!(&b.terminator, Terminator::Assert { cond: Operand::Constant(Const::Bool(false)), expected: true, msg, .. } if msg.starts_with(VIOLATED)))
            .map(|b| b.name)
            .collect();
        for b in &mut f.body.blocks {
            if let Terminator::Call {
                func,
                args,
                target: Some(target),
                ..
            } = &b.terminator
            {
                if args.is_empty() && is_cold_path(func) && never.contains(target) {
                    b.terminator = Terminator::Goto(*target);
                }
            }
        }
    }
}

/// A shift's type, which a precondition check of unchecked_shl compares with the bits: u32.
fn shift_ty(s: &Operand, params: &[(Local, Ty)], decls: &[Decl]) -> Option<IntTy> {
    match s {
        Operand::Constant(Const::Int { ty, .. }) => Some(*ty),
        Operand::Copy(p) | Operand::Move(p) => {
            let l = p.as_local()?;
            let ty = params
                .iter()
                .find(|(x, _)| *x == l)
                .map(|(_, t)| t)
                .or_else(|| decls.iter().find(|d| d.local == l).map(|d| &d.ty))?;
            match ty {
                Ty::Int(t) => Some(*t),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The bits of an integer type.
fn bits(t: IntTy) -> i128 {
    match t {
        IntTy::I8 | IntTy::U8 => 8,
        IntTy::I16 | IntTy::U16 => 16,
        IntTy::I32 | IntTy::U32 => 32,
        IntTy::I64 | IntTy::U64 | IntTy::Isize | IntTy::Usize => 64,
        IntTy::I128 | IntTy::U128 => 128,
    }
}

/// A value, as the walk through a test's block and the block it leads to follows it: what a local held when the
/// walk began, what a line worked out (each its own), or a constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Value {
    Entry(Local),
    Made(usize),
    Const(i128),
}

/// Each local's value, as the lines walked so far leave it; and of the values made, those that are an overflow
/// check's flag: the op and its operands' values.
#[derive(Default)]
struct Values {
    of: HashMap<Local, Value>,
    made: usize,
    sums: HashMap<usize, (BinOp, Option<Value>, Option<Value>)>,
    flags: HashMap<usize, (BinOp, Option<Value>, Option<Value>)>,
}

impl Values {
    fn get(&self, o: &Operand) -> Option<Value> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => {
                let l = p.as_local()?;
                Some(self.of.get(&l).copied().unwrap_or(Value::Entry(l)))
            }
            Operand::Constant(Const::Int { value, .. }) => Some(Value::Const(*value)),
            _ => None,
        }
    }

    /// Walk a statement; false for one the walk can't follow: one that writes through a reference or takes one,
    /// which can change a local without naming it, or one the IR doesn't model.
    fn step(&mut self, s: &Statement) -> bool {
        let Statement::Assign(p, r) = s else {
            return !matches!(s, Statement::Unsupported(_));
        };
        if matches!(r, Rvalue::Ref(..)) || p.projection.contains(&Projection::Deref) {
            return false;
        }
        let Some(l) = p.base_local() else { return false };
        let copied = match (p.as_local(), r) {
            (Some(_), Rvalue::Use(o)) => self.get(o),
            _ => None,
        };
        let v = copied.unwrap_or_else(|| {
            self.made += 1;
            Value::Made(self.made)
        });
        if let (Some(_), Value::Made(k)) = (p.as_local(), v) {
            match r {
                Rvalue::BinaryOp(op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow), x, y) => {
                    self.sums.insert(k, (*op, self.get(x), self.get(y)));
                }
                // the flag of a sum: `f = copy (t.1: bool)`
                Rvalue::Use(Operand::Copy(q) | Operand::Move(q))
                    if q.projection == [Projection::Field(1, Ty::Bool)] =>
                {
                    let t = q.base_local().and_then(|t| self.of.get(&t).copied());
                    if let Some(Value::Made(j)) = t {
                        if let Some(sum) = self.sums.get(&j).cloned() {
                            self.flags.insert(k, sum);
                        }
                    }
                }
                _ => {}
            }
        }
        self.of.insert(l, v);
        true
    }
}

/// What holds where block `bi` starts, from the test that is its only way in: `a < b` (the values of a and b), and
/// the values as the end of its statements leaves them.
fn entry_fact(blocks: &[Block], bi: usize) -> Option<(Values, Value, Value)> {
    let name = blocks[bi].name;
    let ways_in: Vec<usize> = (0..blocks.len())
        .filter(|&i| blocks[i].terminator.successors().contains(&name))
        .collect();
    let [p] = ways_in.as_slice() else { return None };
    let pb = &blocks[*p];
    let Terminator::SwitchInt {
        discr: Operand::Copy(c) | Operand::Move(c),
        arms,
        otherwise: Some(yes),
    } = &pb.terminator
    else {
        return None;
    };
    let ([(0, no)], Some(c)) = (arms.as_slice(), c.as_local()) else {
        return None;
    };
    if *yes != name || *no == name {
        return None;
    }
    let mut v = Values::default();
    let mut fact = None;
    for s in &pb.statements {
        let lt = match s {
            Statement::Assign(d, Rvalue::BinaryOp(BinOp::Lt, a, b)) if d.as_local() == Some(c) => {
                Some((v.get(a)?, v.get(b)?))
            }
            _ => None,
        };
        if !v.step(s) {
            return None;
        }
        if let Some((a, b)) = lt {
            fact = Some((v.of[&c], a, b));
        }
    }
    // the switch tests what the comparison made
    let (made, a, b) = fact?;
    if v.of.get(&c) != Some(&made) {
        return None;
    }
    for s in &blocks[bi].statements {
        if !v.step(s) {
            return None;
        }
    }
    Some((v, a, b))
}

/// Does the step `op(x, 1)` stay within the type, where `a < b`? `a + 1` does (it is at most b), and `b - 1` (it is
/// at least a): a `for` loop's step, which `next()` takes only once its test has passed.
fn step_holds(op: BinOp, x: Option<Value>, one: Option<Value>, a: Value, b: Value) -> bool {
    one == Some(Value::Const(1))
        && match op {
            BinOp::AddWithOverflow => x == Some(a),
            BinOp::SubWithOverflow => x == Some(b),
            _ => false,
        }
}

/// The precondition checks a test proves: those of a `for` loop's step, as rustc inlines `next()` (at opt-level 2).
/// The test `start < end` leads to `unchecked_add(start, 1)`'s check, which then goes, its block joined with the one
/// after it as they are without debug assertions; and `end - 1` of a reversed range likewise. A signed range steps
/// by an overflow-checked add, its overflow way into `unreachable_unchecked`'s check: that way is `unreachable`.
fn proved(blocks: &mut Vec<Block>) {
    let mut bi = 0;
    while bi < blocks.len() {
        let Terminator::Call {
            func,
            args,
            target: Some(target),
            ..
        } = &blocks[bi].terminator
        else {
            bi += 1;
            continue;
        };
        let (target, proves) = match (check(func, false), args.as_slice(), entry_fact(blocks, bi)) {
            (Some((Check::Overflow(op, _), _)), [x, y], Some((v, a, b))) => {
                (*target, step_holds(op, v.get(x), v.get(y), a, b))
            }
            _ => (*target, false),
        };
        if !proves {
            bi += 1;
            continue;
        }
        blocks[bi].terminator = Terminator::Goto(target);
        let ways_in = blocks
            .iter()
            .filter(|b| b.terminator.successors().contains(&target))
            .count();
        if let Some(ti) = blocks
            .iter()
            .position(|b| b.name == target)
            .filter(|&ti| ways_in == 1 && ti != bi)
        {
            let t = blocks.remove(ti);
            let bi = if ti < bi { bi - 1 } else { bi };
            blocks[bi].statements.extend(t.statements);
            blocks[bi].terminator = t.terminator;
        }
    }
    // a signed range's step: its overflow way, through a cold path, into a check that never holds
    let never = |b: &Block| matches!(&b.terminator, Terminator::Call { func, args, .. } if args.is_empty() && matches!(check(func, false), Some((Check::Never, _))));
    let cold = |b: &Block, to: BasicBlock| {
        b.statements.iter().all(|s| {
            matches!(
                s,
                Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop
            )
        }) && matches!(&b.terminator, Terminator::Call { func, args, target: Some(t), .. }
                if args.is_empty() && *t == to && is_cold_path(func))
    };
    let ways_in = |blocks: &[Block], n: BasicBlock| -> Vec<usize> {
        (0..blocks.len())
            .filter(|&i| blocks[i].terminator.successors().contains(&n))
            .collect()
    };
    for di in 0..blocks.len() {
        if !never(&blocks[di]) {
            continue;
        }
        let d = blocks[di].name;
        // the way in: straight from the step, or through its cold path
        let [w] = ways_in(blocks, d)[..] else { continue };
        let (bi, way) = if cold(&blocks[w], d) {
            let [b] = ways_in(blocks, blocks[w].name)[..] else {
                continue;
            };
            (b, blocks[w].name)
        } else {
            (w, d)
        };
        let Terminator::SwitchInt {
            discr: Operand::Copy(f) | Operand::Move(f),
            arms,
            otherwise: Some(over),
        } = &blocks[bi].terminator
        else {
            continue;
        };
        let ([(0, on)], Some(f)) = (arms.as_slice(), f.as_local()) else {
            continue;
        };
        if *over != way || *on == way {
            continue;
        }
        let Some((v, a, b)) = entry_fact(blocks, bi) else {
            continue;
        };
        let Some(Value::Made(k)) = v.of.get(&f).copied() else {
            continue;
        };
        if v.flags.get(&k).is_some_and(|&(op, x, y)| step_holds(op, x, y, a, b)) {
            blocks[di].terminator = Terminator::Unreachable;
        }
    }
}

/// `core::intrinsics::cold_path`, which a std crate's MIR prints as `std::intrinsics::cold_path` (this pass comes
/// before the front end makes std's paths core's).
fn is_cold_path(func: &Path) -> bool {
    func.is_path(&["core", "intrinsics", "cold_path"]) || func.is_path(&["std", "intrinsics", "cold_path"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir;

    fn callee(path: &str) -> Path {
        let line = format!("_3 = {path}(copy _2) -> [return: bb1, unwind unreachable];");
        match ir::parse_line(&line) {
            Ok(ir::Line::Terminator(Terminator::Call { func, .. })) => func,
            other => panic!("not a call: {other:?}"),
        }
    }

    #[test]
    fn the_checks_are_found_by_their_paths_with_rusts_messages() {
        let known = [
            (
                "core::num::<impl usize>::unchecked_add::precondition_check",
                "usize::unchecked_add cannot overflow",
            ),
            (
                "core::num::<impl i16>::unchecked_neg::precondition_check",
                "i16::unchecked_neg cannot overflow",
            ),
            (
                "<usize as SliceIndex<[T]>>::get_unchecked_mut::precondition_check",
                "slice::get_unchecked_mut requires that the index is within the slice",
            ),
            (
                "assert_unchecked::precondition_check",
                "hint::assert_unchecked must never be called when the condition is false",
            ),
            (
                "unreachable_unchecked::precondition_check",
                "hint::unreachable_unchecked must never be reached",
            ),
        ];
        for (path, what) in known {
            let (_, msg) = check(&callee(path), false).unwrap_or_else(|| panic!("{path} not found"));
            assert_eq!(msg, format!("{VIOLATED}{what}"));
        }
        for path in [
            "core::num::<impl usize>::unchecked_add",
            "precondition_check",
            "kit::precondition_check",
        ] {
            assert!(check(&callee(path), false).is_none(), "{path}");
        }
    }

    fn file(body: &str) -> File {
        ir::parse_file(&format!("fn main() -> () {{\n    let mut _0: ();\n{body}\n}}\n"))
    }

    fn terminators(f: &File) -> Vec<String> {
        let Item::Fn(f) = &f.items[0] else { panic!("no fn") };
        f.body.blocks.iter().map(|b| b.terminator.to_string()).collect()
    }

    #[test]
    fn a_for_loops_step_is_proved_and_another_sum_is_checked() {
        let step = "
    bb0: {
        _3 = copy _1;
        _2 = Lt(move _3, const 5_usize);
        switchInt(move _2) -> [0: bb2, otherwise: bb1];
    }
    bb1: {
        _4 = copy _1;
        _5 = core::num::<impl usize>::unchecked_add::precondition_check(copy _4, const 1_usize) -> [return: bb3, unwind unreachable];
    }
    bb2: {
        return;
    }
    bb3: {
        _1 = AddUnchecked(copy _4, const 1_usize);
        goto -> bb0;
    }";
        let mut f = file(step);
        precondition_checks(&mut f, true);
        assert_eq!(
            terminators(&f),
            [
                "switchInt(move _2) -> [0: bb2, otherwise: bb1];",
                "goto -> bb0;",
                "return;"
            ]
        );
        // the same step of a value the test didn't compare: checked
        let mut f = file(&step.replace("_4 = copy _1;", "_4 = copy _6;"));
        precondition_checks(&mut f, true);
        let t = terminators(&f);
        assert!(
            t[1].starts_with(
                "assert(!move (_7.1: bool), \"unsafe precondition(s) violated: usize::unchecked_add cannot overflow\""
            ),
            "{}",
            t[1]
        );
    }
}
