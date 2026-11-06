//! A function that calls itself other than at its end (`n + sum(n - 1)`), with a stack of its own. CARDIAC has
//! none: each function's locals are fixed cells, so the call inside would write over the values the caller still
//! needs. Here such a function calls itself no more: each call of itself is the locals it still reads afterwards
//! pushed, onto an array for each, its arguments into its parameters, and a jump to its start; each return, when
//! the stack isn't empty, pops them back and goes on after the call, the result in the call's place. Where it calls
//! itself in more than one place, a stack of numbers says which place to go back to. A call that would push past the
//! stack's end panics (`stack overflow`, as Rust's does on a stack it overflows), before it writes anything.
//!
//! Only a function whose locals and parameters hold no reference or pointer (which could point into the cells
//! being reused) and that returns no struct; each local it pushes a number or a bool. Any other recursion is left
//! as it is, and refused.
use std::collections::{HashMap, HashSet};

use super::blocks::live_after_returning;
use super::ir::{
    ArrayLen, BasicBlock, BinOp, Block, Const, Decl, IntTy, Local, Operand, Place, PlaceBase, Projection, Rvalue,
    Segment, Statement, Terminator, Ty, Unwind,
};
use super::structs::Function;
use super::util::Ordered;

/// The panic of a call past the stack's end.
pub const OVERFLOW: &str = "stack overflow";

/// Each function of `fns` that calls itself, with a stack `depth` calls deep, where it can be; the cells a call
/// deeper takes, in all.
pub fn software_stacks(fns: &mut Ordered<String, Function>, depth: u64) -> usize {
    let names: Vec<String> = fns.keys().filter(|n| *n != "main").cloned().collect();
    names
        .iter()
        .filter_map(|name| with_a_stack(fns.get_mut(name).unwrap(), depth))
        .sum()
}

/// A call of this function by its own name.
fn calls_itself(t: &Terminator, name: &str) -> bool {
    matches!(t, Terminator::Call { func, .. }
        if func.qself.is_none()
            && matches!(func.segments.as_slice(), [Segment::Name { name: n, args: None }] if n == name))
}

/// A type with no reference or pointer in it.
fn no_pointer(t: &Ty) -> bool {
    match t {
        Ty::Int(_) | Ty::Bool | Ty::Unit => true,
        Ty::Array(e, _) => no_pointer(e),
        Ty::Tuple(ts) => ts.iter().all(no_pointer),
        _ => false,
    }
}

/// Where the function returns to after a call of itself: the call's block, where it goes on, and the place of its
/// result.
struct Site {
    block: usize,
    target: BasicBlock,
    dest: Local,
    args: Vec<Operand>,
    /// what is still read after the call, other than its result: pushed before it, popped after
    saved: Vec<Local>,
}

/// The function with a stack `depth` calls deep, where it calls itself and can have one: the stacks it has.
fn with_a_stack(f: &mut Function, depth: u64) -> Option<usize> {
    let name = f.name.clone();
    if !f.body.blocks.iter().any(|b| calls_itself(&b.terminator, &name)) {
        return None;
    }
    let types: HashMap<Local, Ty> = super::front::local_types(&f.params, &f.body.decls);
    let unsupported = f.body.blocks.iter().any(|b| {
        matches!(b.terminator, Terminator::Unsupported(_))
            || b.statements.iter().any(|s| matches!(s, Statement::Unsupported(_)))
    });
    // a result cell of its own read after a call (a struct's field), or the address of a local
    let result_read = f.body.blocks.iter().any(|b| {
        b.statements
            .iter()
            .flat_map(|s| s.places())
            .chain(b.terminator.places())
            .any(|p| matches!(p.base, PlaceBase::Result { .. }))
    });
    let addressed = !super::blocks::addressed_locals(&f.body.blocks).is_empty();
    if unsupported
        || result_read
        || addressed
        || !f.results.is_empty()
        || !no_pointer(&f.ret)
        || !types.values().all(no_pointer)
        || !f.body.blocks.iter().any(|b| b.name == BasicBlock(0))
    {
        return None;
    }
    let results: HashSet<Local> = HashSet::from([Local(0)]);
    let live = live_after_returning(&f.body.blocks, &results);
    let mut sites: Vec<Site> = vec![];
    for (i, b) in f.body.blocks.iter().enumerate() {
        if !calls_itself(&b.terminator, &name) {
            continue;
        }
        let Terminator::Call {
            dest,
            args,
            target: Some(target),
            unwind: Unwind::Continue | Unwind::Unreachable,
            ..
        } = &b.terminator
        else {
            return None;
        };
        let Some(dest) = dest.as_local() else { return None };
        if args.len() != f.params.len() {
            return None;
        }
        // (a `()` holds nothing)
        let holds = |l: &Local| *l != dest && *l != Local::UNIT && types.get(l) != Some(&Ty::Unit);
        let mut saved: Vec<Local> = live[i].last().unwrap().iter().copied().filter(holds).collect();
        saved.sort();
        // each a number of one cell or a bool, one element of its stack
        let cell = |t: Option<&Ty>| matches!(t, Some(Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool));
        if !saved.iter().all(|l| cell(types.get(l))) {
            return None;
        }
        sites.push(Site {
            block: i,
            target: *target,
            dest,
            args: args.clone(),
            saved,
        });
    }
    let mut next_local = types.keys().map(|l| l.0).max().unwrap_or(0) + 1;
    let mut decls = vec![];
    let mut fresh = |ty: Ty| {
        next_local += 1;
        decls.push(Decl {
            local: Local(next_local - 1),
            mutable: true,
            ty,
        });
        Local(next_local - 1)
    };
    let usize_ty = Ty::Int(IntTy::Usize);
    let number = |value: u64, ty: IntTy| {
        Operand::Constant(Const::Int {
            value: value as i128,
            ty,
        })
    };
    let stack_of = |t: &Ty| Ty::Array(Box::new(t.clone()), ArrayLen::Known(depth));
    let at = |stack: Local, sp: Local| Place {
        base: PlaceBase::Local(stack),
        projection: vec![Projection::Index(sp)],
    };
    let sp = fresh(usize_ty.clone());
    // the stacks: a call's locals of a type, the first on the first stack of that type, the next on the next, so
    // that the calls share them; and one of where to go back to when there is more than one place
    let mut stacks: Vec<(Ty, usize, Local)> = vec![];
    let mut homes: Vec<Vec<Local>> = vec![];
    for site in &sites {
        let mut of_type: HashMap<Ty, usize> = HashMap::new();
        let mut on = vec![];
        for l in &site.saved {
            let ty = types[l].clone();
            let k = of_type.entry(ty.clone()).or_default();
            let home = match stacks.iter().find(|(t, j, _)| *t == ty && j == k) {
                Some((_, _, s)) => *s,
                None => {
                    let s = fresh(stack_of(&ty));
                    stacks.push((ty, *k, s));
                    s
                }
            };
            *k += 1;
            on.push(home);
        }
        homes.push(on);
    }
    let ways_back = (sites.len() > 1).then(|| fresh(stack_of(&Ty::Int(IntTy::U8))));
    let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let mut new_block = || {
        next_block += 1;
        BasicBlock(next_block - 1)
    };
    // the entry: an empty stack, then the function's own start (its entry renamed, which each call jumps back to)
    let start = new_block();
    for b in &mut f.body.blocks {
        if b.name == BasicBlock(0) {
            b.name = start;
        }
        for t in b.terminator.targets_mut() {
            if *t == BasicBlock(0) {
                *t = start;
            }
        }
    }
    let entry = Block {
        name: BasicBlock(0),
        statements: vec![Statement::Assign(
            Place::local(sp),
            Rvalue::Use(number(0, IntTy::Usize)),
        )],
        terminator: Terminator::Goto(start),
    };
    let mut extra: Vec<Block> = vec![];
    // each return: out of the function when the stack is empty, else back after the call that is popped
    let out = new_block();
    let pop = new_block();
    for b in &mut f.body.blocks {
        if b.terminator == Terminator::Return {
            b.terminator = Terminator::SwitchInt {
                discr: Operand::Copy(Place::local(sp)),
                arms: vec![(0, out)],
                otherwise: Some(pop),
            };
        }
    }
    // each call: a check that the stack has room, the pushes, the arguments into the parameters, and the jump. With
    // nothing to push, the stack is its count alone, which may count as far as a card does
    let bound = if stacks.is_empty() && ways_back.is_none() {
        999
    } else {
        depth
    };
    let mut back = vec![];
    for (k, site) in sites.iter().enumerate() {
        let room = fresh(Ty::Bool);
        let push = new_block();
        let b = &mut f.body.blocks[site.block];
        b.statements.push(Statement::Assign(
            Place::local(room),
            Rvalue::BinaryOp(BinOp::Lt, Operand::Copy(Place::local(sp)), number(bound, IntTy::Usize)),
        ));
        b.terminator = Terminator::Assert {
            cond: Operand::Move(Place::local(room)),
            expected: true,
            msg: OVERFLOW.into(),
            msg_args: vec![],
            target: push,
            unwind: Unwind::Continue,
        };
        let mut statements: Vec<Statement> = site
            .saved
            .iter()
            .zip(&homes[k])
            .map(|(l, s)| Statement::Assign(at(*s, sp), Rvalue::Use(Operand::Copy(Place::local(*l)))))
            .collect();
        if let Some(w) = ways_back {
            statements.push(Statement::Assign(at(w, sp), Rvalue::Use(number(k as u64, IntTy::U8))));
        }
        statements.push(Statement::Assign(
            Place::local(sp),
            Rvalue::BinaryOp(BinOp::Add, Operand::Copy(Place::local(sp)), number(1, IntTy::Usize)),
        ));
        let mut temps = vec![];
        for (a, (_, ty)) in site.args.iter().zip(&f.params) {
            let t = fresh(ty.clone());
            statements.push(Statement::Assign(Place::local(t), Rvalue::Use(a.clone())));
            temps.push(t);
        }
        for (t, (p, _)) in temps.into_iter().zip(&f.params) {
            statements.push(Statement::Assign(
                Place::local(*p),
                Rvalue::Use(Operand::Move(Place::local(t))),
            ));
        }
        extra.push(Block {
            name: push,
            statements,
            terminator: Terminator::Goto(start),
        });
        // after it: the result in its place, the locals popped, and on
        let mut statements = vec![];
        if f.ret != Ty::Unit {
            statements.push(Statement::Assign(
                Place::local(site.dest),
                Rvalue::Use(Operand::Copy(Place::local(Local(0)))),
            ));
        }
        for (l, s) in site.saved.iter().zip(&homes[k]) {
            statements.push(Statement::Assign(
                Place::local(*l),
                Rvalue::Use(Operand::Copy(at(*s, sp))),
            ));
        }
        let after = new_block();
        extra.push(Block {
            name: after,
            statements,
            terminator: Terminator::Goto(site.target),
        });
        back.push(after);
    }
    extra.push(Block {
        name: out,
        statements: vec![],
        terminator: Terminator::Return,
    });
    let mut statements = vec![Statement::Assign(
        Place::local(sp),
        Rvalue::BinaryOp(BinOp::Sub, Operand::Copy(Place::local(sp)), number(1, IntTy::Usize)),
    )];
    let terminator = match ways_back {
        Some(w) => {
            let which = fresh(Ty::Int(IntTy::U8));
            statements.push(Statement::Assign(
                Place::local(which),
                Rvalue::Use(Operand::Copy(at(w, sp))),
            ));
            let mut arms: Vec<(i128, BasicBlock)> = back.iter().enumerate().map(|(k, b)| (k as i128, *b)).collect();
            let otherwise = arms.pop().map(|(_, b)| b);
            Terminator::SwitchInt {
                discr: Operand::Move(Place::local(which)),
                arms,
                otherwise,
            }
        }
        None => Terminator::Goto(back[0]),
    };
    extra.push(Block {
        name: pop,
        statements,
        terminator,
    });
    f.body.blocks.extend(extra);
    // (the first block is the function's entry)
    f.body.blocks.insert(0, entry);
    f.body.decls.extend(decls);
    Some(stacks.len() + usize::from(ways_back.is_some()))
}
