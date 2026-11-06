//! The compiler: rustc's MIR, as the CARDIAC driver converts it to the IR (`ir`), lowered to CARDIAC assembly.
//!
//! Scope: functions over u8, i16, usize and bool; locals of those types, their (T, bool) overflow pairs, arrays of
//! u8, i16 or bool, structs of scalar fields (and arrays of them), and references to them; and the statements and
//! terminators such programs produce. Anything else is rejected, with the offending MIR line, rather than guessed
//! at. Calls to the built-ins (`BUILTINS`) are instructions: `out` is OUT, `inp` INP, `overlay` ends a phase of a
//! program too big for memory (the loader reads the next phase's code over this one's), `load_resource` is cells the
//! loader fills from a named resource, `digit` a shift built on the spot, and `skip_cards` and `add_times` a jump
//! built into a shared row of `INP 00` or `ADD c`.
//!
//! An i16 lives in one cell, so it is a card value, -999..999 (and a usize is 0..999). CARDIAC has
//! no multiply or divide: `*` by a constant becomes adds and decimal shifts, `/` and `%` by a power of ten become
//! shifts (SFT truncates toward zero, as Rust does), and the rest go to small shared routines, `mul` and `div`,
//! emitted once into any program that needs them.
//!
//! Checks are those of a Rust build: bounds checks always, and overflow checks when the build's profile (or a single
//! file's flags) turns them on, as the MIR rustc writes shows. A failed check panics, which prints -999 on the
//! teletype and halts. Without overflow checks, a value past -999..999 keeps the low three digits when stored, as a
//! cell does.
pub mod arith;
pub mod blocks;
pub mod copies;
pub mod crates;
pub mod enum_methods;
pub mod enums;
pub mod forloops;
pub mod front;
pub mod io;
pub mod ir;
pub mod lower;
pub mod machine;
pub mod methods;
pub mod names;
pub mod panic_messages;
pub mod peephole;
pub mod phases;
pub mod preconditions;
pub mod program;
pub mod ranges;
pub mod runtime;
pub mod stack;
pub mod structs;
pub mod tails;
pub mod hoist;
pub mod util;

use std::collections::{HashMap, HashSet};

use self::lower::{Callees, Lowering, Shared, Sig, Sym};
use self::machine::{Kind, Line};
use self::util::Ordered;

pub use self::phases::{split_phases, Phases};

/// The built-ins: calls to functions of these names, at the root of the program's crate or of a crate it uses (the
/// built-ins crate, `cardiac`), are instructions rather than calls. `assume()` is `core::hint::assert_unchecked`.
pub const BUILTINS: [&str; 7] = [
    "inp",
    "out",
    "overlay",
    "load_resource",
    "digit",
    "skip_cards",
    "add_times",
];

/// Why a program cannot be compiled.
pub type Fail = String;

/// A compiled program: its listing, and the label each phase after the first starts at.
pub struct Compiled {
    pub listing: Vec<Line>,
    /// what the programmer might change, said as a note when it is built (`bool_note`)
    pub advice: Vec<String>,
    pub phase_starts: Vec<String>,
    /// where main could be cut into phases, had it no overlay() (`phases::cut_points`)
    pub cut_points: Vec<phases::Cut>,
    /// the source's names for the locals, for the listing (`names`)
    pub names: names::Names,
}

impl Compiled {
    /// The listing as assembly text.
    pub fn text(&self) -> String {
        machine::print(&self.listing)
    }
}

/// Compile a crate's MIR. With `optimize` off, every statement is lowered as it stands. `debug_assertions`: whether
/// rustc built it with them on, which the MIR doesn't always show (a call of core's `assert_unchecked`).
/// `enums`: the enums the driver looked up, which the MIR names but doesn't define. `auto_overlay`: whether
/// the compiler cuts main into phases itself when it doesn't fit, rather than at the program's own overlay() calls.
pub fn compile_mir(
    mir: &ir::File,
    optimize: bool,
    debug_assertions: bool,
    enums: &[enums::EnumDef],
    auto_overlay: bool,
) -> Result<Compiled, Fail> {
    if let Some(k) = forced_cuts() {
        // the fuzzer's hook: main cut at those of its places, fit or not, to check that any cut runs as one piece
        // does (a program with an overlay() of its own is compiled as it is)
        if let Ok(whole) = compile_cut(mir, optimize, debug_assertions, enums, Some(&[])) {
            let n = whole.cut_points.len();
            let mut cuts: Vec<phases::Cut> = vec![];
            for c in k.iter().filter(|_| n > 0).map(|k| whole.cut_points[k % n]) {
                if !cuts.contains(&c) {
                    cuts.push(c);
                }
            }
            return compile_cut(mir, optimize, debug_assertions, enums, Some(&cuts));
        }
    }
    if !auto_overlay {
        return compile_cut(mir, optimize, debug_assertions, enums, None);
    }
    let whole = compile_cut(mir, optimize, debug_assertions, enums, Some(&[]))?;
    if whole.cut_points.is_empty() || largest_phase(&whole.listing).is_none_or(|n| n <= MEMORY) {
        return Ok(whole);
    }
    // Too big: cut main where it runs once. Each way to cut it is compiled and
    // measured, and the split kept is the smallest in all whose every phase fits; when no one cut is enough, the cut
    // that leaves the largest phase smallest is kept and another is looked for.
    let mut cuts: Vec<phases::Cut> = vec![];
    for _ in 1..MAX_PHASES {
        // the smallest split, and of those the one whose largest phase is smallest: room to grow
        let mut fit: Option<((usize, usize), Compiled)> = None;
        let mut nearest: Option<(usize, phases::Cut)> = None;
        for &way in whole.cut_points.iter().filter(|w| !cuts.contains(w)) {
            let mut tried = cuts.clone();
            tried.push(way);
            let Ok(c) = compile_cut(mir, optimize, debug_assertions, enums, Some(&tried)) else {
                continue;
            };
            let (Some(largest), Ok(r)) = (largest_phase(&c.listing), split_phases(&c.listing)) else {
                continue;
            };
            let total = cells(&r.data) + r.code.iter().map(|p| cells(p)).sum::<usize>();
            if largest <= MEMORY {
                if fit.as_ref().is_none_or(|(t, _)| (total, largest) < *t) {
                    fit = Some(((total, largest), c));
                }
            } else if nearest.is_none_or(|(n, _)| largest < n) {
                nearest = Some((largest, way));
            }
        }
        if let Some((_, mut c)) = fit {
            c.advice.push(format!(
                "main doesn't fit in memory in one piece, so auto-overlay split it into {} phases",
                c.phase_starts.len() + 1
            ));
            return Ok(c);
        }
        let Some((_, way)) = nearest else { break };
        cuts.push(way);
    }
    Ok(whole)
}

/// `CARDIAC_FORCE_CUT=K[,K...]`, for fuzzing: cut main at its Kth places to cut (counted round), whether or not
/// it fits.
fn forced_cuts() -> Option<Vec<usize>> {
    let v = std::env::var("CARDIAC_FORCE_CUT").ok()?;
    v.split(',').map(|k| k.trim().parse().ok()).collect()
}

/// CARDIAC's memory: cells 03 to 98.
const MEMORY: usize = 96;

/// The most phases a program is split into on its own.
const MAX_PHASES: usize = 4;

/// The cells the largest phase fills, with the data every phase shares; none if the listing doesn't split.
fn largest_phase(listing: &[Line]) -> Option<usize> {
    let r = split_phases(listing).ok()?;
    r.code.iter().map(|c| cells(&r.data) + cells(c)).max()
}

/// The compile, with main cut into phases at `cuts`; None: at its own overlay() calls, auto-overlay off.
fn compile_cut(
    mir: &ir::File,
    optimize: bool,
    debug_assertions: bool,
    enums: &[enums::EnumDef],
    cuts: Option<&[phases::Cut]>,
) -> Result<Compiled, Fail> {
    // parameters passed straight on share the argument's cell (copies::forwarded), kept only when that is no bigger
    let (shared, forwards) = compile_niched(mir, optimize, true, debug_assertions, enums, cuts)?;
    if !forwards {
        return Ok(shared);
    }
    let plain = compile_niched(mir, optimize, false, debug_assertions, enums, cuts)?.0;
    Ok(if cells(&shared.listing) <= cells(&plain.listing) {
        shared
    } else {
        plain
    })
}

/// core::hint::assert_unchecked(c): the program's promise that c holds (false would be undefined behaviour).
/// Optimized, it gives the range pass a fact and costs nothing; otherwise it is checked, as an assert. The same
/// promise, once rustc has inlined assert_unchecked (opt-level 2), is an `assume(b)` statement mid-block: the block
/// is cut there, so the promise ends a block as an assert does. A call to one of `promisers` is the same promise.
fn promises(blocks: &mut Vec<ir::Block>, optimize: bool, promisers: &HashSet<String>) {
    let promise = |v: ir::Local, target: ir::BasicBlock| ir::Terminator::Assert {
        cond: ir::Operand::Move(ir::Place::local(v)),
        expected: true,
        msg: "assumed".into(),
        msg_args: vec![],
        target,
        unwind: ir::Unwind::Continue,
    };
    for b in blocks.iter_mut() {
        if let Some((v, target)) = promise_call(&b.terminator, promisers) {
            b.terminator = promise(v, target);
        }
    }
    let mut cut = 0;
    let mut i = 0;
    while i < blocks.len() {
        let assume = blocks[i].statements.iter().position(|s| matches!(s, ir::Statement::Assume(ir::Operand::Copy(p) | ir::Operand::Move(p)) if p.as_local().is_some()));
        if let Some(k) = assume {
            let ir::Statement::Assume(o) = &blocks[i].statements[k] else {
                unreachable!()
            };
            let v = o.local().unwrap();
            let name = ir::BasicBlock(800000 + cut);
            cut += 1;
            let statements = blocks[i].statements.split_off(k + 1);
            blocks[i].statements.pop();
            let terminator = std::mem::replace(&mut blocks[i].terminator, promise(v, name));
            blocks.insert(
                i + 1,
                ir::Block {
                    name,
                    statements,
                    terminator,
                },
            );
        }
        i += 1;
    }
    if optimize {
        joined_promises(blocks);
    }
}

/// A call that is a promise, of `assert_unchecked` or one of `promisers`: the local it promises, and where it goes on.
fn promise_call(t: &ir::Terminator, promisers: &HashSet<String>) -> Option<(ir::Local, ir::BasicBlock)> {
    let ir::Terminator::Call {
        dest,
        func,
        args,
        target: Some(target),
        unwind: ir::Unwind::Continue | ir::Unwind::Unreachable,
    } = t
    else {
        return None;
    };
    let plain_path = func.qself.is_none()
        && func
            .segments
            .iter()
            .all(|s| matches!(s, ir::Segment::Name { args: None, .. }));
    match (
        dest.as_local().filter(|l| *l != ir::Local::UNIT),
        plain_path,
        func.name() == "assert_unchecked" || promisers.contains(func.name()),
        args.as_slice(),
    ) {
        (Some(_), true, true, [ir::Operand::Move(p) | ir::Operand::Copy(p)]) => Some((p.as_local()?, *target)),
        _ => None,
    }
}

/// The functions that do nothing but promise their one parameter: `cardiac::assume` where rustc has not inlined it
/// (opt-level 1 and s), whose body is `assert_unchecked`. A call to one is a promise, as `assert_unchecked` is, and
/// no call: the function is compiled only if something calls it otherwise. Built with debug assertions, `assume`'s
/// body is an `assert!` instead, a check that stays: so it is the body that tells, not the name.
fn promisers(fns: &Ordered<String, structs::Function>) -> HashSet<String> {
    let promises_only = |f: &structs::Function| {
        let ([(p, ir::Ty::Bool)], ir::Ty::Unit) = (f.params.as_slice(), &f.ret) else {
            return false;
        };
        let mut blocks = f.body.blocks.clone();
        promises(&mut blocks, false, &HashSet::new());
        // a promise of p, as `promises` makes it
        let promised = |t: &ir::Terminator| match t {
            ir::Terminator::Assert { target, .. } => {
                *t == promise_of(ir::Operand::Move(ir::Place::local(*p)), true, *target)
            }
            _ => false,
        };
        let idle = |s: &ir::Statement| {
            matches!(
                s,
                ir::Statement::StorageLive(_) | ir::Statement::StorageDead(_) | ir::Statement::Nop
            )
        };
        blocks.iter().any(|b| promised(&b.terminator))
            && blocks.iter().all(|b| {
                b.statements.iter().all(idle)
                    && (promised(&b.terminator)
                        || matches!(b.terminator, ir::Terminator::Goto(_) | ir::Terminator::Return))
            })
    };
    fns.iter()
        .filter(|(_, f)| promises_only(f))
        .map(|(n, _)| n.clone())
        .collect()
}

/// A promise of `a && b`: rustc's MIR works the bool out through a branch (`v = b` on one way in, `v = const
/// false` on the other) before it is promised. The way that sets it false is undefined, so the test that leads
/// there is itself a promise; and each way in that sets `v` then jumps to the promise gets a copy of it, beside the
/// comparison, where the range pass can read it. Either way, each operand of the `&&` is a promise of its own.
fn joined_promises(blocks: &mut [ir::Block]) {
    let promised = |t: &ir::Terminator| match t {
        ir::Terminator::Assert {
            cond: ir::Operand::Move(p),
            expected: true,
            msg,
            msg_args,
            ..
        } if msg == "assumed" && msg_args.is_empty() => p.as_local(),
        _ => None,
    };
    let sets = |s: &ir::Statement, v: ir::Local| matches!(s, ir::Statement::Assign(p, _) if p.as_local() == Some(v));
    // a bool whose address is taken can change unseen
    let taken: HashSet<ir::Local> = blocks
        .iter()
        .flat_map(|b| &b.statements)
        .filter_map(|s| match s {
            ir::Statement::Assign(_, ir::Rvalue::Ref(_, p)) => p.base_local(),
            _ => None,
        })
        .collect();
    let preds = ir::predecessors(blocks);
    // the blocks that jump to a promise of v with v false, and those that jump to it having just set v
    let mut never: HashSet<ir::BasicBlock> = HashSet::new();
    let mut copies: Vec<(usize, ir::Local, ir::BasicBlock)> = vec![];
    for b in blocks.iter() {
        let Some(v) = promised(&b.terminator).filter(|v| !taken.contains(v)) else {
            continue;
        };
        if b.statements.iter().any(|s| sets(s, v)) {
            continue;
        }
        for p in preds.get(&b.name).into_iter().flatten() {
            let k = blocks.iter().position(|x| x.name == *p).unwrap();
            if blocks[k].terminator != ir::Terminator::Goto(b.name) {
                continue;
            }
            match blocks[k].statements.iter().rev().find(|s| sets(s, v)) {
                Some(ir::Statement::Assign(_, ir::Rvalue::Use(ir::Operand::Constant(ir::Const::Bool(false))))) => {
                    never.insert(*p);
                }
                Some(_) => copies.push((k, v, b.name)),
                None => {}
            }
        }
    }
    for (k, v, target) in copies {
        if !never.contains(&blocks[k].name) {
            blocks[k].terminator = promise_of(ir::Operand::Move(ir::Place::local(v)), true, target);
        }
    }
    for b in blocks.iter_mut() {
        if let ir::Terminator::SwitchInt {
            discr: discr @ (ir::Operand::Copy(_) | ir::Operand::Move(_)),
            arms,
            otherwise: Some(yes),
        } = &b.terminator
        {
            if let [(0, no)] = arms.as_slice() {
                let promise = match (never.contains(no), never.contains(yes)) {
                    (true, false) => Some((true, *yes)),
                    (false, true) => Some((false, *no)),
                    _ => None,
                };
                if let Some((expected, target)) = promise {
                    b.terminator = promise_of(discr.clone(), expected, target);
                }
            }
        }
    }
}

/// A promise that `cond` is `expected`, going on to `target`.
fn promise_of(cond: ir::Operand, expected: bool, target: ir::BasicBlock) -> ir::Terminator {
    ir::Terminator::Assert {
        cond,
        expected,
        msg: "assumed".into(),
        msg_args: vec![],
        target,
        unwind: ir::Unwind::Continue,
    }
}

/// A comparison with a constant just past what a card holds, `x < 1000`, is the same test as one with the last
/// value it holds, `x <= 999`, which can be on a card: so it is written that way. (The constant on either side; a
/// constant one past the bottom, `x > -1000`, likewise.)
fn card_bounds(blocks: &mut [ir::Block]) {
    use ir::BinOp::{Ge, Gt, Le, Lt};
    for s in blocks.iter_mut().flat_map(|b| b.statements.iter_mut()) {
        let ir::Statement::Assign(_, ir::Rvalue::BinaryOp(op, x, y)) = s else {
            continue;
        };
        // the constant, which side it is on, and the type's range
        let (c, right) = match (&mut *x, &mut *y) {
            (_, ir::Operand::Constant(c @ ir::Const::Int { .. })) => (c, true),
            (ir::Operand::Constant(c @ ir::Const::Int { .. }), _) => (c, false),
            _ => continue,
        };
        let ir::Const::Int { value, ty } = c else {
            unreachable!()
        };
        let Some([lo, hi]) = ranges::type_range(&ir::Ty::Int(*ty)) else {
            continue;
        };
        // x op c, or c op x as x op' c
        let flip = |o: ir::BinOp| match o {
            Lt => Gt,
            Le => Ge,
            Gt => Lt,
            Ge => Le,
            o => o,
        };
        let o = if right { *op } else { flip(*op) };
        let (o, v) = match o {
            Lt if *value == hi as i128 + 1 => (Le, hi),
            Ge if *value == hi as i128 + 1 => (Gt, hi),
            Gt if *value == lo as i128 - 1 => (Ge, lo),
            Le if *value == lo as i128 - 1 => (Lt, lo),
            _ => continue,
        };
        *value = v as i128;
        *op = if right { o } else { flip(o) };
    }
}

/// Each local's type, for the passes to look up.
fn local_types(locals: &Ordered<ir::Local, ir::Ty>) -> HashMap<ir::Local, ir::Ty> {
    locals.iter().map(|(l, t)| (*l, t.clone())).collect()
}

/// An array or a slice, which never lives in the scratch pool (nor does a type the IR does not model that is
/// written as one).
fn array_like(ty: &ir::Ty) -> bool {
    match ty {
        ir::Ty::Array(..) | ir::Ty::Slice(_) => true,
        ir::Ty::Other(text) => text.starts_with('['),
        _ => false,
    }
}

/// The note that a program's bools would be smaller as `cardiac::Flag`s: each test of a bool kept in a cell
/// is `CLA b / SUB 00 / TAC`, and of a Flag `CLA f / TAC`; a Flag's false needs a -1 cell, which the program may
/// have already. Said only when that saves 2 cells or more.
fn bool_note(tests: &[Option<String>], listing: &[Line]) -> Option<String> {
    let has_minus_one = machine::print(listing).lines().any(|l| l.starts_with("kn1:"));
    let saved = tests.len().checked_sub(if has_minus_one { 0 } else { 1 })?;
    if saved < 2 {
        return None;
    }
    let mut names: Vec<&str> = vec![];
    for n in tests.iter().flatten() {
        if !names.contains(&n.as_str()) {
            names.push(n);
        }
    }
    let which = match names.as_slice() {
        [] => "bools kept in cells are".to_string(),
        [one] => format!("the bool `{one}` is"),
        more => format!(
            "the bools {} are",
            more.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")
        ),
    };
    Some(format!(
        "{which} tested {} times; as cardiac::Flag (`false.into()`, `.get()`) each test is an instruction less: \
         about {saved} cells or more",
        tests.len()
    ))
}

/// The cells a listing fills: one for each instruction and each data cell.
pub fn cells(lines: &[Line]) -> usize {
    lines
        .iter()
        .filter(|l| matches!(l.kind, Kind::Instr(..) | Kind::Dat(..)))
        .count()
}

/// The compile, with the niche layout for each enum shaped as `Option<i16>` whose guards the range pass proves
/// (`enums::Niche`): compiled with every such niche, then again without each whose guard stayed, until none stays.
fn compile_niched(
    file: &ir::File,
    optimize: bool,
    forward: bool,
    debug_assertions: bool,
    enums: &[enums::EnumDef],
    cuts: Option<&[phases::Cut]>,
) -> Result<(Compiled, bool), Fail> {
    let mut no_niche = HashSet::new();
    loop {
        let (compiled, forwards, unproved) =
            match compile(file, optimize, forward, debug_assertions, enums, &no_niche, cuts) {
                Ok(c) => c,
                // (with its guards, say, the program may not fit where it fits without them): again with no niche
                Err(_) if !no_niche.contains(enums::NO_NICHES) => {
                    no_niche = HashSet::from([enums::NO_NICHES.to_string()]);
                    continue;
                }
                Err(e) => return Err(e),
            };
        if unproved.is_empty() {
            return Ok((compiled, forwards));
        }
        no_niche.extend(unproved);
    }
}

/// The compile, with or without forwarded parameters; also whether any parameter was forwarded, and the niches
/// whose guards the range pass didn't prove (`no_niche`: those to lay out with a tag cell).
fn compile(
    file: &ir::File,
    optimize: bool,
    forward: bool,
    debug_assertions: bool,
    enums: &[enums::EnumDef],
    no_niche: &HashSet<String>,
    cuts: Option<&[phases::Cut]>,
) -> Result<(Compiled, bool, HashSet<String>), Fail> {
    let (file, mut fns, named) = front::front_end(file.clone(), debug_assertions, enums)?;
    if !fns.has("main") {
        return Err("no fn main in MIR".into());
    }
    let names: Vec<String> = fns.keys().cloned().collect();
    // rustc's u8 + - and * wrap, before the passes write plain ops of their own
    for n in &names {
        let f = fns.get_mut(n).unwrap();
        let types: HashMap<ir::Local, ir::Ty> = f
            .params
            .iter()
            .cloned()
            .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
            .collect();
        ranges::mark_wrapping(&mut f.body.blocks, &types);
    }
    for n in &names {
        forloops::lower_for_loops(fns.get_mut(n).unwrap());
        // a range's contains(): its tests, while the range is a struct the struct pass will take apart
        methods::inline_range_contains(fns.get_mut(n).unwrap());
    }
    // enums with data as structs of their cells, which the struct pass takes apart
    // Option's and Result's methods that rustc calls at opt-level 1 and s: the tests it inlines at 2
    let normalized = enums::normalized(enums);
    // each closure's function, by the closure's type: its first parameter's
    let closures: HashMap<String, String> = fns
        .iter()
        .filter_map(|(name, f)| match f.params.first() {
            Some((_, ir::Ty::Other(t))) if t.starts_with("{closure@") => Some((t.clone(), name.clone())),
            _ => None,
        })
        .collect();
    for n in &names {
        enum_methods::inline_checked(fns.get_mut(n).unwrap());
        enum_methods::inline_option_eq(fns.get_mut(n).unwrap(), &normalized);
        enum_methods::inline_enum_methods(fns.get_mut(n).unwrap(), &normalized, &closures);
    }
    // a closure that captures nothing is called without itself: its first parameter, never read, goes
    for name in closures.values() {
        let f = fns.get_mut(name).unwrap();
        let unread = f
            .params
            .first()
            .is_some_and(|(l, _)| ir::occurrences(&f.body.blocks, *l) == 0);
        if unread {
            f.params.remove(0);
        }
    }
    let known = enums::data_enums(&mut fns, enums, no_niche)?;
    structs::scalarize_structs(&mut fns, &file, known)?;
    // abs, min and their kin, which rustc calls at opt-level 1 and s: the code it inlines at 2
    for n in &names {
        methods::inline_methods(fns.get_mut(n).unwrap());
        methods::inline_operators(fns.get_mut(n).unwrap());
    }
    // the teletype, the card reader and the loader are not functions
    for f in ["out", "inp", "overlay"] {
        fns.remove(f);
    }
    let mut sigs: Ordered<String, Sig> = Ordered::new();
    let mut calls: Ordered<String, Vec<String>> = Ordered::new();
    let builtin: Vec<&str> = BUILTINS.iter().copied().chain(["assert_unchecked"]).collect();
    let promisers = promisers(&fns);
    for (name, f) in fns.iter() {
        sigs.set(
            name.clone(),
            Sig {
                params: f.params.iter().map(|(l, _)| *l).collect(),
                widths: f
                    .params
                    .iter()
                    .map(|(_, ty)| match ty {
                        ir::Ty::Array(_, ir::ArrayLen::Known(length)) => *length,
                        _ => 1,
                    })
                    .collect(),
                ret: f.ret.clone(),
            },
        );
        // the functions it calls by one plain name, as they come in its body (not a promise: that is no call)
        let called = f
            .body
            .blocks
            .iter()
            .filter(|b| promise_call(&b.terminator, &promisers).is_none());
        let called = called.filter_map(|b| match &b.terminator {
            ir::Terminator::Call {
                func, target: Some(_), ..
            } if func.qself.is_none() && matches!(func.segments.as_slice(), [ir::Segment::Name { args: None, .. }]) => {
                Some(func.name().to_string())
            }
            _ => None,
        });
        calls.set(
            name.clone(),
            called.filter(|n| !builtin.contains(&n.as_str())).collect(),
        );
    }
    // only what main can reach, and no cycles: a fixed set of cells per function cannot hold recursion
    let mut order: Vec<String> = vec![];
    fn visit(
        n: &str,
        path: &mut Vec<String>,
        fns_calls: &Ordered<String, Vec<String>>,
        seen: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), Fail> {
        if path.iter().any(|p| p == n) {
            return Err(format!(
                "recursion ({} -> {n}) needs a stack; this backend has none yet",
                path.join(" -> ")
            ));
        }
        if seen.contains(n) {
            return Ok(());
        }
        let Some(cs) = fns_calls.get(n) else {
            return Err(format!("call to unknown function {n}"));
        };
        path.push(n.to_string());
        for c in cs.clone() {
            visit(&c, path, fns_calls, seen, order)?;
        }
        path.pop();
        seen.insert(n.to_string());
        order.push(n.to_string());
        Ok(())
    }
    visit("main", &mut vec![], &calls, &mut HashSet::new(), &mut order)?;
    let mut sh = Shared {
        pool: Default::default(),
        consts: vec![],
        phase_starts: vec![],
        named,
        resources: vec![],
        bool_tests: vec![],
    };
    struct Ready {
        name: String,
        locals: Ordered<ir::Local, ir::Ty>,
        blocks: Vec<ir::Block>,
        pooled: HashSet<ir::Local>,
        names: Ordered<String, ir::Local>,
        debug: Vec<ir::Debug>,
        merged: Vec<(ir::Local, ir::Local)>,
        params: Vec<ir::Local>,
        results: HashSet<ir::Local>,
    }
    let mut ready: Vec<Ready> = vec![];
    let mut cut_points = vec![];
    let mut unproved: HashSet<String> = HashSet::new();
    for n in std::iter::once("main".to_string()).chain(order.iter().filter(|x| *x != "main").cloned()) {
        let f = fns.get(&n).unwrap();
        let body = f.body.clone();
        // each local's type, the parameters' first, in the order they are declared (the order of their cells)
        let mut locals: Ordered<ir::Local, ir::Ty> = Ordered::new();
        for (l, t) in &f.params {
            locals.set(*l, t.clone());
        }
        for d in &body.decls {
            locals.set(d.local, d.ty.clone());
        }
        let mut blocks = body.blocks;
        let named: HashSet<ir::Local> = body
            .debug
            .iter()
            .filter_map(|d| d.named_local())
            .map(|(_, l)| l)
            .collect();
        let params: HashSet<ir::Local> = f.params.iter().map(|(l, _)| *l).collect();
        let results: HashSet<ir::Local> = f.results.iter().copied().collect();
        card_bounds(&mut blocks);
        blocks::settled_checks(&mut blocks, &results);
        if optimize {
            copies::forward_copies(&mut blocks, &local_types(&locals), &named, &params);
        }
        promises(&mut blocks, optimize, &promisers);
        // the locals that came to share another's cell, for the names the listing gives them
        let mut merged: Vec<(ir::Local, ir::Local)> = vec![];
        if optimize {
            // what nothing reads goes first: a variable that only grows needs no range, and an op whose value is
            // never read is left only for its panic, which the range pass may rule out
            blocks::not_of_comparisons(&mut blocks);
            blocks::dead_code(&mut blocks, &results);
            blocks::bools_on_edges(&mut blocks, true);
            blocks::thread_constants(&mut blocks);
            ranges::range_pass(&mut blocks, &local_types(&locals))?;
            blocks::dead_code(&mut blocks, &results);
            blocks::dead_stores(&mut blocks, &results);
            blocks::merge_blocks(&mut blocks);
            // a counter that only counts counts to 0, the test CARDIAC has
            let fixed: HashSet<ir::Local> = params.iter().chain(&results).chain([&ir::Local(0)]).copied().collect();
            for c in blocks::rebase_counters(&mut blocks, &local_types(&locals), &fixed, &params) {
                locals.set(c, ir::Ty::Int(ir::IntTy::I16));
            }
            // what rustc copied into both ways of a test it answers again, joined once each copy has its checks
            tails::join_tails(&mut blocks, &mut locals);
            arith::rematerialize(&mut blocks, &mut locals);
            arith::merge_repeats(&mut blocks);
            copies::forward_fields(&mut blocks, &named);
            let types = local_types(&locals);
            arith::sum_chains(&mut blocks, &types);
            blocks::merge_blocks(&mut blocks);
            arith::lin_fold(&mut blocks, &types, &named);
            // again: the range pass turns proven casts into copies
            copies::forward_copies(&mut blocks, &local_types(&locals), &named, &params);
            // what a loop works out the same every time round, once before it (CARDIAC_HOIST=1 while measured)
            if hoist::enabled() {
                let fixed: HashSet<ir::Local> = params.iter().chain(&results).chain([&ir::Local(0)]).copied().collect();
                hoist::hoist_invariants(&mut blocks, &fixed);
            }
            io::io_pass(&mut blocks);
            let keep: HashSet<ir::Local> = std::iter::once(ir::Local(0)).chain(results.iter().copied()).collect();
            copies::join_copies(&mut blocks, &local_types(&locals), &keep);
            let into = keep.clone();
            let keep: HashSet<ir::Local> = keep.iter().copied().chain(params.iter().copied()).collect();
            merged = copies::share_cells(&mut blocks, &mut locals, &keep, &into, &named);
            // sharing a cell can make a move of one local a read of the other, still to be read; and the merges
            // above can make a test of a comparison read again later
            blocks::copy_unless_last(&mut blocks);
            blocks::bools_on_edges(&mut blocks, false);
        }
        // main cut into phases (auto-overlay), before anything is decided block by block: a local defined and used in
        // one block is scratch (`pooled`), which a cut through the block would leave in a cell the next phase doesn't
        // read
        if let (true, Some(cuts)) = (n == "main", cuts) {
            if phases::has_overlay(&blocks) {
                return Err(
                    "overlay() in main with auto-overlay on: the compiler places the phases itself. Take the \
                            overlay() out, or turn auto-overlay off to place them by hand"
                        .into(),
                );
            }
            cut_points = phases::cut_points(&blocks);
            phases::cut(&mut blocks, cuts);
        }
        // a niche's guard the range pass didn't prove
        for b in &blocks {
            if let ir::Terminator::Assert { msg, .. } = &b.terminator {
                if let Some(strukt) = msg.strip_prefix(enums::NICHE_GUARD) {
                    unproved.insert(strukt.to_string());
                }
            }
        }
        let mut pooled = HashSet::new();
        if optimize {
            for (&l, ty) in locals.iter() {
                if l == ir::Local(0) || named.contains(&l) || params.contains(&l) || array_like(ty) {
                    continue;
                }
                let homes: Vec<&ir::Block> = blocks.iter().filter(|b| b.lines().any(|x| x.mentions(l))).collect();
                if homes.len() != 1 {
                    continue;
                }
                // defined before any use, inside one block
                let first = homes[0].each_line().find(|x| x.locals().contains(&l)).unwrap();
                let dest = match first {
                    blocks::Line::Statement(ir::Statement::Assign(p, _)) => p.as_local(),
                    blocks::Line::Terminator(ir::Terminator::Call { dest, .. }) => dest.as_local(),
                    _ => None,
                };
                if dest == Some(l) {
                    pooled.insert(l);
                }
            }
        }
        // a local no line names needs no cell, whatever its type: rustc leaves code it never reaches out of the
        // MIR but keeps its locals (a `Range` of a `for` under `if false`), and the passes leave locals unread
        let named_in_code: HashSet<ir::Local> = blocks
            .iter()
            .flat_map(|b| b.each_line().flat_map(|x| x.locals()))
            .collect();
        locals.retain(|l, _| {
            *l == ir::Local(0) || params.contains(l) || results.contains(l) || named_in_code.contains(l)
        });
        let names: Ordered<String, ir::Local> = {
            let mut o = Ordered::new();
            for (name, l) in body.debug.iter().filter_map(|d| d.named_local()) {
                o.set(name.to_string(), l);
            }
            o
        };
        if n == "main" {
            phases::order_phases(&mut blocks)?;
        }
        let params = sigs.get(&n).map(|s| s.params.clone()).unwrap_or_default();
        ready.push(Ready {
            name: n,
            locals,
            blocks,
            pooled,
            names,
            debug: body.debug.clone(),
            merged,
            params,
            results,
        });
    }
    // callees first, so a parameter can be passed on through more than one call
    let mut alias: HashMap<Sym, Sym> = HashMap::new();
    if optimize && forward {
        for n in order.iter() {
            let r = ready.iter().find(|r| &r.name == n).unwrap();
            let types = local_types(&r.locals);
            for (from, to) in copies::forwarded(n, &r.blocks, &r.params, &r.results, &types, &sigs, &alias) {
                alias.insert(from, to);
            }
        }
    }
    let note = |text: &str| Line::new(Kind::Note(text.to_string()));
    let mut listing = vec![
        note(" CARDIAC assembly compiled from rustc MIR"),
        note(" entry: main_bb0"),
    ];
    let mut data: Vec<Line> = vec![];
    let callees = Callees::new(sigs.iter(), &alias);
    let source_names = names::Names::of(ready.iter().map(|r| (r.name.as_str(), r.debug.as_slice(), r.merged.as_slice())));
    for r in ready {
        let shown = source_names.of_function(&r.name);
        let huge = ranges::huge_usizes(&r.blocks, &local_types(&r.locals), &r.results, None);
        let mut l = Lowering::new(&r.name, r.locals, &mut sh, &callees, optimize, r.pooled)?;
        l.names = r.names;
        l.huge = huge;
        let (mut code, cells) = l.lower(r.blocks)?;
        names::comments(&mut code, &shown);
        listing.extend(code);
        data.extend(cells);
    }
    let (routines, runtime_data) = runtime::runtime(&sh.pool, &sh.consts);
    listing.extend(routines);
    data.extend(runtime_data);
    listing.push(Line::new(Kind::DataStart));
    // an empty data section is still one (blank) line, as it was when the listing was joined from text
    if data.is_empty() {
        data.push(Line::new(Kind::Blank));
    }
    listing.extend(data);
    listing.push(Line::new(Kind::Blank));
    let starts = sh.phase_starts.clone();
    let listing = if optimize {
        let mut l = peephole::peephole(&peephole::layout(&peephole::peephole(&listing, &starts)), &starts);
        l = stack::share_cells(&l);
        // a local and the argument cell it is passed in, made one cell, leave copies of a cell onto itself: those
        // alone go (the other rules know a scratch cell's uses as they were before cells were shared)
        l = peephole::drop_self_copies(&l);
        l.push(Line::new(Kind::Blank));
        l
    } else {
        listing
    };
    let advice = bool_note(&sh.bool_tests, &listing).into_iter().collect();
    Ok((
        Compiled {
            listing,
            advice,
            phase_starts: starts,
            cut_points,
            names: source_names,
        },
        !alias.is_empty(),
        unproved,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::machine::parse;

    const WITHOUT_MINUS_ONE: &str = "start:\n        HRS 0\n\nk0:       DAT 0\n";
    const WITH_MINUS_ONE: &str = "start:\n        HRS 0\n\nkn1:      DAT -1\n";

    fn note(tests: &[Option<&str>], listing: &str) -> Option<String> {
        let tests: Vec<Option<String>> = tests.iter().map(|t| t.map(str::to_string)).collect();
        bool_note(&tests, &parse(listing).unwrap())
    }

    /// Three tests less the -1 the program needs: 2 cells, said, each name once.
    #[test]
    fn bool_note_names_each_bool_once() {
        let n = note(&[Some("a"), Some("b"), Some("a")], WITHOUT_MINUS_ONE).unwrap();
        assert!(n.starts_with("the bools `a`, `b` are tested 3 times;"), "{n}");
        assert!(n.ends_with("about 2 cells or more"), "{n}");
    }

    /// Two tests, but the -1 costs one: 1 cell, not worth a note. With the -1 there already, 2.
    #[test]
    fn bool_note_counts_the_minus_one() {
        assert_eq!(note(&[Some("a"), Some("a")], WITHOUT_MINUS_ONE), None);
        let n = note(&[Some("a"), None], WITH_MINUS_ONE).unwrap();
        assert!(n.starts_with("the bool `a` is tested 2 times;"), "{n}");
        assert_eq!(note(&[], WITH_MINUS_ONE), None);
    }
}
