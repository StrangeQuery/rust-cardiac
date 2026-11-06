//! Functions that call one another as the last thing each does: `is_even(n)` returning `is_odd(n - 1)`, which
//! returns `is_even(n - 1)`. As a function's call of itself at its end is a loop (`front::tail_calls`), so is such a
//! cycle: its functions are one function, with a start for each of them, and each of their calls of one another is
//! the arguments stored in the parameters of the one it calls and a jump to that one's start. No call is left
//! inside the cycle, so it needs no stack.
//!
//! Only where every call between the functions of the cycle is a call whose result is the caller's own, with nothing
//! after it but the way to the return, all of them return the same type, and no parameter or argument is a
//! reference or a pointer (which could point into cells being reused). Any other cycle is left as it is, and the
//! compiler refuses it as recursion.
use std::collections::{HashMap, HashSet};

use super::ir::visit_mut::{walk_place, VisitMut};
use super::ir::{
    BasicBlock, Block, Const, Debug, Decl, File, FnItem, IntTy, Item, ItemPath, Local, Operand, Path, Place,
    Projection, Rvalue, Segment, Statement, Terminator, Ty, Unwind,
};

/// Each cycle of the program's functions whose calls of one another are all at their ends, as one function. A
/// function the rest of the program calls is a way in: when there is only one, the merged function is that one, under
/// its name, the others written into it; when there are more, it is a function of its own, named after them all
/// (`is_even_is_odd`), whose first parameter says which of them was called, and every call of one of them from
/// outside the cycle is a call of it.
pub(super) fn mutual_tail_calls(file: &mut File) {
    let index: HashMap<String, usize> = file
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            Item::Fn(f) if f.path.is_plain() && !f.unread() => Some((f.path.to_string(), i)),
            _ => None,
        })
        .collect();
    // what each function calls, by item
    let calls: HashMap<usize, HashSet<usize>> = index
        .values()
        .map(|&i| {
            let Item::Fn(f) = &file.items[i] else { unreachable!() };
            let called = f.body.blocks.iter().filter_map(|b| match &b.terminator {
                Terminator::Call { func, .. } if func.is_plain() => index.get(&func.to_string()).copied(),
                _ => None,
            });
            (i, called.collect())
        })
        .collect();
    let reach = |from: usize| {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut todo = vec![from];
        while let Some(i) = todo.pop() {
            for &c in &calls[&i] {
                if seen.insert(c) {
                    todo.push(c);
                }
            }
        }
        seen
    };
    let reaches: HashMap<usize, HashSet<usize>> = index.values().map(|&i| (i, reach(i))).collect();
    let mut order: Vec<usize> = index.values().copied().collect();
    order.sort();
    let mut done: HashSet<usize> = HashSet::new();
    let mut merged = vec![];
    for &i in &order {
        if done.contains(&i) {
            continue;
        }
        // the functions that reach this one and that it reaches: its cycle (a function calling only itself is
        // `front::tail_calls`'s)
        let cycle: Vec<usize> = order
            .iter()
            .copied()
            .filter(|j| reaches[&i].contains(j) && reaches[j].contains(&i))
            .collect();
        done.extend(&cycle);
        if cycle.len() < 2 {
            continue;
        }
        if let Some(m) = merge(file, &cycle) {
            merged.push((cycle, m));
        }
    }
    let mut names: HashSet<String> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(f) => Some(f.path.to_string()),
            _ => None,
        })
        .collect();
    let mut gone: HashSet<usize> = HashSet::new();
    let mut ways_in = vec![];
    for (cycle, mut m) in merged {
        // a new name is one no function has
        if m.selector {
            while names.contains(&m.item.path.to_string()) {
                m.item.path.segments.last_mut().unwrap().push('_');
            }
            names.insert(m.item.path.to_string());
        }
        file.items[cycle[0]] = Item::Fn(m.item.clone());
        gone.extend(&cycle[1..]);
        ways_in.push(m);
    }
    // each call of one of a cycle's functions (from outside it: none is left inside), of the merged function at that
    // one's start
    for (k, item) in file.items.iter_mut().enumerate() {
        let Item::Fn(f) = item else { continue };
        if gone.contains(&k) {
            continue;
        }
        for b in &mut f.body.blocks {
            let Terminator::Call { func, args, .. } = &mut b.terminator else {
                continue;
            };
            for m in &ways_in {
                if let Some(at) = m.names.iter().position(|n| func.is_plain() && *n == func.to_string()) {
                    *func = path(&m.item.path);
                    *args = m.entry_args(at, std::mem::take(args));
                    break;
                }
            }
        }
    }
    let mut k = 0;
    file.items.retain(|_| {
        k += 1;
        !gone.contains(&(k - 1))
    });
}

/// A cycle's functions as one.
struct Merged {
    item: FnItem,
    /// the functions' names, in the cycle's order
    names: Vec<String>,
    /// whether the first parameter says which function was called
    selector: bool,
    /// for each function, the merged function's parameter that holds each of its own
    slots: Vec<Vec<usize>>,
}

impl Merged {
    /// The arguments of a call from outside the cycle of function `at`, given that call's own arguments: which one
    /// it is, then each parameter's value, a zero for a parameter of another of the functions.
    fn entry_args(&self, at: usize, args: Vec<Operand>) -> Vec<Operand> {
        let mut out: Vec<Option<Operand>> = vec![None; self.item.params.len()];
        let first = usize::from(self.selector);
        for (a, &s) in args.into_iter().zip(&self.slots[at]) {
            out[first + s] = Some(a);
        }
        if self.selector {
            out[0] = Some(Operand::Constant(Const::Int {
                value: at as i128,
                ty: IntTy::U8,
            }));
        }
        out.into_iter()
            .zip(&self.item.params)
            .map(|(a, (_, ty))| a.unwrap_or_else(|| Operand::Constant(zero(ty).unwrap())))
            .collect()
    }
}

/// The zero of a parameter's type, for a call that doesn't use the parameter: of a number or a bool only.
fn zero(ty: &Ty) -> Option<Const> {
    match ty {
        Ty::Int(t) => Some(Const::Int { value: 0, ty: *t }),
        Ty::Bool => Some(Const::Bool(false)),
        _ => None,
    }
}

/// A call's path to a function of this name.
fn path(p: &ItemPath) -> Path {
    Path {
        qself: None,
        segments: p
            .segments
            .iter()
            .map(|s| Segment::Name {
                name: s.clone(),
                args: None,
            })
            .collect(),
    }
}

/// From a block on, only blocks that do nothing until the return (`StorageDead`s, `goto`s).
pub(super) fn goes_to_return(start: BasicBlock, blocks: &[Block]) -> bool {
    let mut at = start;
    for _ in 0..blocks.len() {
        let Some(b) = blocks.iter().find(|b| b.name == at) else {
            return false;
        };
        let idle = b.statements.iter().all(|s| {
            matches!(
                s,
                Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop
            )
        });
        match (&b.terminator, idle) {
            (Terminator::Return, true) => return true,
            (Terminator::Goto(next), true) => at = *next,
            _ => return false,
        }
    }
    false
}

fn pointer(t: &Ty) -> bool {
    matches!(t, Ty::Ref { .. } | Ty::Ptr { .. })
}

/// The functions of `cycle` (items of the file, in its order) as one, or None where they can't be.
fn merge(file: &File, cycle: &[usize]) -> Option<Merged> {
    let fns: Vec<&FnItem> = cycle
        .iter()
        .map(|&i| match &file.items[i] {
            Item::Fn(f) => f,
            _ => unreachable!(),
        })
        .collect();
    let names: Vec<String> = fns.iter().map(|f| f.path.to_string()).collect();
    let member = |func: &Path| {
        func.is_plain()
            .then(|| names.iter().position(|n| *n == func.to_string()))
            .flatten()
    };
    // the ways in, from the rest of the program
    let mut entered: Vec<usize> = vec![];
    for (k, item) in file.items.iter().enumerate() {
        let Item::Fn(f) = item else { continue };
        if cycle.contains(&k) {
            continue;
        }
        for b in &f.body.blocks {
            if let Terminator::Call { func, .. } = &b.terminator {
                if let Some(m) = member(func) {
                    if !entered.contains(&m) {
                        entered.push(m);
                    }
                }
            }
        }
    }
    entered.sort();
    let (&first, ret) = (entered.first()?, &fns[0].ret);
    let unsupported = |f: &FnItem| {
        f.body.blocks.iter().any(|b| {
            matches!(b.terminator, Terminator::Unsupported(_))
                || b.statements.iter().any(|s| matches!(s, Statement::Unsupported(_)))
        })
    };
    let plain = |f: &&FnItem| {
        &f.ret == ret
            && f.params.iter().all(|(_, t)| !pointer(t))
            && f.body.blocks.iter().any(|b| b.name == BasicBlock(0))
            && !unsupported(f)
            && f.path.to_string() != "main"
    };
    if !fns.iter().all(plain) {
        return None;
    }
    // every call within the cycle is at its caller's end
    for f in &fns {
        let types: HashMap<Local, Ty> = super::front::local_types(&f.params, &f.body.decls);
        for b in &f.body.blocks {
            let Terminator::Call {
                dest,
                func,
                args,
                target,
                unwind,
            } = &b.terminator
            else {
                continue;
            };
            let Some(callee) = member(func) else { continue };
            let arg_pointer = |o: &Operand| match o {
                Operand::Copy(p) | Operand::Move(p) => super::front::place_ty(&types, p).is_none_or(|t| pointer(&t)),
                Operand::Constant(_) => false,
                _ => true,
            };
            let at_end = dest.as_local() == Some(Local(0))
                && target.is_some_and(|t| goes_to_return(t, &f.body.blocks))
                && matches!(unwind, Unwind::Continue | Unwind::Unreachable)
                && args.len() == fns[callee].params.len()
                && !args.iter().any(arg_pointer);
            if !at_end {
                return None;
            }
        }
    }
    // the merged function's own order: the first way in, then the rest as the file has them
    let by: Vec<usize> = std::iter::once(first)
        .chain((0..fns.len()).filter(|&k| k != first))
        .collect();
    let selector = entered.len() > 1;
    let top = fns[first]
        .params
        .iter()
        .map(|(l, _)| l.0)
        .chain(fns[first].body.decls.iter().map(|d| d.local.0))
        .max()
        .unwrap_or(0);
    let mut next_local = top + 1;
    let mut fresh = || {
        next_local += 1;
        Local(next_local - 1)
    };
    let sel = selector.then(&mut fresh);
    // the parameters: the first way in's own, and for each other function's parameter one of its type that the
    // function hasn't taken already, or one more
    let mut params: Vec<(Local, Ty)> = fns[first].params.clone();
    let mut slots: Vec<Vec<usize>> = vec![vec![]; fns.len()];
    slots[first] = (0..params.len()).collect();
    for &k in &by[1..] {
        for (_, t) in &fns[k].params {
            let free = (0..params.len()).find(|s| params[*s].1 == *t && !slots[k].contains(s));
            let s = free.unwrap_or_else(|| {
                params.push((fresh(), t.clone()));
                params.len() - 1
            });
            slots[k].push(s);
        }
    }
    // a call from outside passes a zero for a parameter not its function's: only a number's or a bool's
    let passed_by_all = |s: usize| entered.iter().all(|&k| slots[k].contains(&s));
    if (0..params.len()).any(|s| !passed_by_all(s) && zero(&params[s].1).is_none()) {
        return None;
    }
    // each function's locals as the merged function's: the first way in's as they are
    let mut locals: Vec<HashMap<Local, Local>> = vec![HashMap::new(); fns.len()];
    let mut decls: Vec<Decl> = fns[first].body.decls.clone();
    for &k in &by[1..] {
        let map = &mut locals[k];
        map.insert(Local(0), Local(0));
        for ((l, _), &s) in fns[k].params.iter().zip(&slots[k]) {
            map.insert(*l, params[s].0);
        }
        for d in &fns[k].body.decls {
            if d.local == Local(0) {
                continue;
            }
            let to = fresh();
            map.insert(d.local, to);
            decls.push(Decl {
                local: to,
                mutable: d.mutable,
                ty: d.ty.clone(),
            });
        }
    }
    // each function's blocks: its entry a start of its own (the merged entry chooses among them), the first way in's
    // others as they are
    let mut next_block = fns[first].body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let mut new_block = || {
        next_block += 1;
        BasicBlock(next_block - 1)
    };
    let mut blocks_of: Vec<HashMap<BasicBlock, BasicBlock>> = vec![HashMap::new(); fns.len()];
    for &k in &by {
        for b in &fns[k].body.blocks {
            let to = if k == first && b.name != BasicBlock(0) {
                b.name
            } else {
                new_block()
            };
            blocks_of[k].insert(b.name, to);
        }
    }
    let starts: Vec<BasicBlock> = (0..fns.len()).map(|k| blocks_of[k][&BasicBlock(0)]).collect();
    let mut blocks: Vec<Block> = vec![Block {
        name: BasicBlock(0),
        statements: vec![],
        terminator: match sel {
            Some(sel) => {
                let mut arms: Vec<(i128, BasicBlock)> = entered.iter().map(|&k| (k as i128, starts[k])).collect();
                let otherwise = arms.pop().map(|(_, b)| b);
                Terminator::SwitchInt {
                    discr: Operand::Copy(Place::local(sel)),
                    arms,
                    otherwise,
                }
            }
            None => Terminator::Goto(starts[first]),
        },
    }];
    let mut debug: Vec<Debug> = vec![];
    let mut inlined = vec![];
    for &k in &by {
        let f = fns[k];
        let mut body = f.body.clone();
        if k != first {
            Renumber(&locals[k]).visit_body(&mut body);
            for b in &mut body.blocks {
                for s in &mut b.statements {
                    if let Statement::StorageLive(l) | Statement::StorageDead(l) = s {
                        *l = locals[k][l];
                    }
                }
            }
        }
        for b in &mut body.blocks {
            b.name = blocks_of[k][&b.name];
            for t in b.terminator.targets_mut() {
                *t = blocks_of[k][t];
            }
        }
        // each call of one of the cycle: its arguments into temporaries, then into the callee's parameters, and on
        // to its start
        for b in &mut body.blocks {
            let Terminator::Call { func, args, .. } = &b.terminator else {
                continue;
            };
            let Some(callee) = member(func) else { continue };
            let mut temps = vec![];
            for (a, (_, ty)) in args.clone().into_iter().zip(&fns[callee].params) {
                let t = fresh();
                decls.push(Decl {
                    local: t,
                    mutable: false,
                    ty: ty.clone(),
                });
                b.statements.push(Statement::Assign(Place::local(t), Rvalue::Use(a)));
                temps.push(t);
            }
            for (t, &s) in temps.into_iter().zip(&slots[callee]) {
                b.statements.push(Statement::Assign(
                    Place::local(params[s].0),
                    Rvalue::Use(Operand::Move(Place::local(t))),
                ));
            }
            b.terminator = Terminator::Goto(starts[callee]);
        }
        blocks.extend(body.blocks);
        if k == first {
            debug.extend(body.debug);
            inlined.extend(body.inlined);
        } else {
            // a name of a whole local only: another's is text, which can't be renumbered (it only names the cell)
            debug.extend(body.debug.iter().filter_map(|d| {
                let (name, l) = d.named_local()?;
                Some(Debug {
                    name: name.to_string(),
                    value: locals[k].get(&l)?.to_string(),
                })
            }));
            inlined.extend(
                body.inlined
                    .iter()
                    .filter_map(|(l, p)| Some((*locals[k].get(l)?, p.clone()))),
            );
        }
    }
    let path = if selector {
        let mut segments = fns[first].path.segments.clone();
        let last = segments.len() - 1;
        segments[last] = fns
            .iter()
            .map(|f| f.path.segments.last().unwrap().as_str())
            .collect::<Vec<_>>()
            .join("_");
        ItemPath { segments }
    } else {
        fns[first].path.clone()
    };
    let params = sel.map(|s| (s, Ty::Int(IntTy::U8))).into_iter().chain(params).collect();
    Some(Merged {
        item: FnItem {
            path,
            params,
            ret: ret.clone(),
            body: super::ir::Body {
                decls,
                debug,
                blocks,
                inlined,
            },
        },
        names,
        selector,
        slots,
    })
}

/// Each local, as the merged function's.
struct Renumber<'a>(&'a HashMap<Local, Local>);

impl VisitMut for Renumber<'_> {
    fn visit_place(&mut self, p: &mut Place) {
        if let Some(l) = p.base_local().and_then(|l| self.0.get(&l)) {
            p.base = Place::local(*l).base;
        }
        for x in p.projection.iter_mut() {
            if let Projection::Index(l) | Projection::IndexPlus(l, _) = x {
                if let Some(to) = self.0.get(l) {
                    *l = *to;
                }
            }
        }
        walk_place(self, p)
    }
}
