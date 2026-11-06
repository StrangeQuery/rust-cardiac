//! Core's slice methods that rustc leaves as calls, at every opt-level: `split_at`, `copy_from_slice`,
//! `rotate_left`, `binary_search` and the insertion sort `sort_unstable` calls for a short slice. There is no MIR here
//! for core's own bodies, and they are written for a machine with words and pointers (`rotate_left` moves blocks
//! through a buffer on the stack), so each call becomes the plain loop over the slice that does the same, as blocks of
//! the caller's: written with the slice's length (`PtrMetadata`), its elements (`(*s)[i]`) and slices made of a
//! pointer and a length (`SliceFromParts`), which `slices::run_time_lengths` then lays out. Each checks what core
//! checks, with core's message, and gives what core gives: `binary_search` is core's own search, step for step, so
//! that on a slice that isn't sorted it finds the element core finds.
use std::collections::HashMap;

use super::ir::{
    self, BasicBlock, BinOp, Block, Const, IntTy, Local, Operand, Path, Place, PlaceBase, Projection, Rvalue, Segment,
    Statement, Terminator, Ty, Unwind,
};
use super::structs::Function;

/// The element type of a pointer to a slice or an array, and whether it is mutable.
fn element(t: &Ty) -> Option<(Ty, bool)> {
    let (Ty::Ref { to, mutable, .. } | Ty::Ptr { to, mutable }) = t else {
        return None;
    };
    match &**to {
        Ty::Slice(e) | Ty::Array(e, _) if matches!(**e, Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::I8 | IntTy::Usize)) => {
            Some(((**e).clone(), *mutable))
        }
        _ => None,
    }
}

/// Is it a method of `[T]` (`slice::<impl [i16]>::split_at`) of this name?
fn slice_method(p: &Path, name: &str) -> bool {
    matches!(p.segments.as_slice(), [Segment::Name { name: s, .. }, Segment::Impl(Ty::Slice(_)), Segment::Name { name: m, .. }] if s == "slice" && m == name)
}

/// The segments' names, generic arguments left out, of a path with no `<T as Trait>::`.
fn names(p: &Path) -> Vec<&str> {
    p.segments
        .iter()
        .map(|s| match s {
            Segment::Name { name, .. } => name.as_str(),
            Segment::Impl(_) => "",
        })
        .collect()
}

fn usize_const(n: u64) -> Operand {
    Operand::Constant(Const::Int {
        value: n as i128,
        ty: IntTy::Usize,
    })
}

fn copy(l: Local) -> Operand {
    Operand::Copy(Place::local(l))
}

/// `(*s)[i]`
fn element_at(s: Local, i: Local) -> Place {
    Place {
        base: PlaceBase::Local(s),
        projection: vec![Projection::Deref, Projection::Index(i)],
    }
}

/// The blocks a call becomes, written one after another: each statement goes in the block being written, and a
/// terminator ends it.
struct Writer {
    local: u32,
    block: u32,
    decls: Vec<ir::Decl>,
    blocks: Vec<Block>,
    at: BasicBlock,
    statements: Vec<Statement>,
}

impl Writer {
    fn local(&mut self, ty: Ty) -> Local {
        let l = Local(self.local);
        self.local += 1;
        self.decls.push(ir::Decl {
            local: l,
            mutable: true,
            ty,
        });
        l
    }

    fn name(&mut self) -> BasicBlock {
        let b = BasicBlock(self.block);
        self.block += 1;
        b
    }

    fn set(&mut self, p: Place, r: Rvalue) {
        self.statements.push(Statement::Assign(p, r));
    }

    /// A fresh local of type `ty`, set to r.
    fn fresh(&mut self, ty: Ty, r: Rvalue) -> Local {
        let l = self.local(ty);
        self.set(Place::local(l), r);
        l
    }

    fn usize(&mut self, r: Rvalue) -> Local {
        self.fresh(Ty::Int(IntTy::Usize), r)
    }

    fn test(&mut self, op: BinOp, a: Operand, b: Operand) -> Local {
        self.fresh(Ty::Bool, Rvalue::BinaryOp(op, a, b))
    }

    /// End the block being written with t, and go on writing `next`.
    fn end(&mut self, t: Terminator, next: BasicBlock) {
        let statements = std::mem::take(&mut self.statements);
        self.blocks.push(Block {
            name: self.at,
            statements,
            terminator: t,
        });
        self.at = next;
    }

    /// End the last block, with t.
    fn last(&mut self, t: Terminator) {
        let statements = std::mem::take(&mut self.statements);
        self.blocks.push(Block {
            name: self.at,
            statements,
            terminator: t,
        });
    }

    /// Go to `to`, and write `next`.
    fn goto(&mut self, to: BasicBlock, next: BasicBlock) {
        self.end(Terminator::Goto(to), next);
    }

    /// On to `yes` where c holds, else `no`; write `next`.
    fn branch(&mut self, c: Local, yes: BasicBlock, no: BasicBlock, next: BasicBlock) {
        let t = Terminator::SwitchInt {
            discr: Operand::Move(Place::local(c)),
            arms: vec![(0, no)],
            otherwise: Some(yes),
        };
        self.end(t, next);
    }

    /// A panic with core's message unless c holds; then on, in a block of its own.
    fn check(&mut self, c: Local, msg: &str) {
        let next = self.name();
        let t = Terminator::Assert {
            cond: Operand::Move(Place::local(c)),
            expected: true,
            msg: msg.into(),
            msg_args: vec![],
            target: next,
            unwind: Unwind::Continue,
        };
        self.end(t, next);
    }

    fn len(&mut self, s: Local) -> Local {
        let meta = Path {
            qself: None,
            segments: vec![Segment::Name {
                name: "PtrMetadata".into(),
                args: None,
            }],
        };
        self.usize(Rvalue::Variant(meta, vec![copy(s)]))
    }

    /// `i += 1`
    fn step(&mut self, i: Local) {
        self.set(Place::local(i), Rvalue::BinaryOp(BinOp::Add, copy(i), usize_const(1)));
    }
}

/// The slice an operand passes: its local, element type and mutability.
fn slice_arg(types: &HashMap<Local, Ty>, o: &Operand) -> Option<(Local, Ty, bool)> {
    let l = o.local()?;
    let (e, m) = element(types.get(&l)?)?;
    Some((l, e, m))
}

/// The length of an array pointer's array.
fn array_len(t: &Ty) -> Option<u64> {
    let (Ty::Ref { to, .. } | Ty::Ptr { to, .. }) = t else {
        return None;
    };
    match &**to {
        Ty::Array(_, ir::ArrayLen::Known(n)) => Some(*n),
        _ => None,
    }
}

/// The slice type of elements e.
fn slice_of(e: &Ty, mutable: bool) -> Ty {
    Ty::Ref {
        lifetime: None,
        mutable,
        to: Box::new(Ty::Slice(Box::new(e.clone()))),
    }
}

/// `SliceFromParts(p, n)`
fn from_parts(p: Local, n: Operand) -> Rvalue {
    let name = Segment::Name {
        name: "SliceFromParts".into(),
        args: None,
    };
    Rvalue::Variant(
        Path {
            qself: None,
            segments: vec![name],
        },
        vec![copy(p), n],
    )
}

/// `split_at(s, k)`: k checked against the length, then the slices on either side of it, the pair's two fields
/// wherever the pair is read. Only where the pair is read field by field.
fn split_at(w: &mut Writer, types: &HashMap<Local, Ty>, args: &[Operand], ret: BasicBlock) -> Option<(Local, Local)> {
    let [s, k] = args else { return None };
    let (s, e, mutable) = slice_arg(types, s)?;
    let n = w.len(s);
    // (k read once: an argument passed by move)
    let k = w.usize(Rvalue::Use(k.clone()));
    let ok = w.test(BinOp::Le, copy(k), copy(n));
    w.check(ok, "mid > len");
    let p = w.fresh(
        Ty::Ptr {
            mutable,
            to: Box::new(e.clone()),
        },
        Rvalue::Use(copy(s)),
    );
    let a = w.fresh(slice_of(&e, mutable), from_parts(p, copy(k)));
    let q = w.fresh(
        Ty::Ptr {
            mutable,
            to: Box::new(e.clone()),
        },
        Rvalue::BinaryOp(BinOp::Add, copy(p), copy(k)),
    );
    let m = w.usize(Rvalue::BinaryOp(BinOp::Sub, copy(n), copy(k)));
    let b = w.fresh(slice_of(&e, mutable), from_parts(q, copy(m)));
    w.last(Terminator::Goto(ret));
    Some((a, b))
}

/// `copy_from_slice(dst, src)`: the lengths checked equal, then the elements copied one by one.
fn copy_from_slice(w: &mut Writer, types: &HashMap<Local, Ty>, args: &[Operand], ret: BasicBlock) -> Option<()> {
    let [d, s] = args else { return None };
    let ((d, e, true), (s, f, _)) = (slice_arg(types, d)?, slice_arg(types, s)?) else {
        return None;
    };
    if e != f {
        return None;
    }
    let (n, m) = (w.len(d), w.len(s));
    let same = w.test(BinOp::Eq, copy(n), copy(m));
    w.check(
        same,
        "copy_from_slice: source slice length does not match destination slice length",
    );
    let i = w.usize(Rvalue::Use(usize_const(0)));
    let (head, body) = (w.name(), w.name());
    w.goto(head, head);
    let more = w.test(BinOp::Lt, copy(i), copy(n));
    w.branch(more, body, ret, body);
    let x = w.fresh(e, Rvalue::Use(Operand::Copy(element_at(s, i))));
    w.set(element_at(d, i), Rvalue::Use(copy(x)));
    w.step(i);
    w.last(Terminator::Goto(head));
    Some(())
}

/// `rotate_left(s, k)`: k checked against the length, then k turns of the slice by one, each its first element
/// put last.
fn rotate_left(w: &mut Writer, types: &HashMap<Local, Ty>, args: &[Operand], ret: BasicBlock) -> Option<()> {
    let [s, k] = args else { return None };
    let (s, e, true) = slice_arg(types, s)? else {
        return None;
    };
    let n = w.len(s);
    let k = w.usize(Rvalue::Use(k.clone()));
    let ok = w.test(BinOp::Le, copy(k), copy(n));
    w.check(ok, "assertion failed: mid <= self.len()");
    let (outer, turn, inner, shift, last) = (w.name(), w.name(), w.name(), w.name(), w.name());
    w.goto(outer, outer);
    let more = w.test(BinOp::Gt, copy(k), usize_const(0));
    w.branch(more, turn, ret, turn);
    // a turn: the first element kept, each after it moved down one, the first put last (k > 0, so the slice has one)
    let at = |offset: u64| Place {
        base: PlaceBase::Local(s),
        projection: vec![
            Projection::Deref,
            Projection::ConstantIndex {
                offset,
                min_length: offset + 1,
            },
        ],
    };
    let first = w.fresh(e.clone(), Rvalue::Use(Operand::Copy(at(0))));
    let end = w.usize(Rvalue::BinaryOp(BinOp::Sub, copy(n), usize_const(1)));
    let j = w.usize(Rvalue::Use(usize_const(0)));
    w.goto(inner, inner);
    let more = w.test(BinOp::Lt, copy(j), copy(end));
    w.branch(more, shift, last, shift);
    let next = w.usize(Rvalue::BinaryOp(BinOp::Add, copy(j), usize_const(1)));
    let x = w.fresh(e, Rvalue::Use(Operand::Copy(element_at(s, next))));
    w.set(element_at(s, j), Rvalue::Use(copy(x)));
    w.set(Place::local(j), Rvalue::Use(copy(next)));
    w.goto(inner, last);
    let end_at = match types.get(&s).and_then(array_len) {
        Some(n) => at(n - 1),
        None => element_at(s, end),
    };
    w.set(end_at, Rvalue::Use(copy(first)));
    w.set(Place::local(k), Rvalue::BinaryOp(BinOp::Sub, copy(k), usize_const(1)));
    w.last(Terminator::Goto(outer));
    Some(())
}

/// `insertion_sort_shift_left(v, 1, is_less)` with `is_less` the element type's `<`, which `sort_unstable` calls for
/// a slice of 2 to 20 elements: each element from the second on moved down past those above it. (Core aborts for
/// an offset of 0 or past the length, which `sort_unstable` never passes.)
fn insertion_sort(
    w: &mut Writer,
    types: &HashMap<Local, Ty>,
    func: &Path,
    args: &[Operand],
    ret: BasicBlock,
) -> Option<()> {
    let [v, offset, _] = args else { return None };
    let (v, e, true) = slice_arg(types, v)? else {
        return None;
    };
    if offset.int() != Some((1, IntTy::Usize)) {
        return None;
    }
    // the comparison is the element type's own `<`
    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
        return None;
    };
    let lt = [
        format!("{{<{e} as PartialOrd>::lt}}"),
        format!("{{<{e} as core::cmp::PartialOrd>::lt}}"),
    ];
    if !g
        .args
        .iter()
        .any(|a| matches!(a, ir::GenericArg::Ty(Ty::Other(t)) if lt.iter().any(|lt| t.ends_with(lt.as_str()))))
    {
        return None;
    }
    let n = w.len(v);
    let i = w.usize(Rvalue::Use(usize_const(1)));
    let (outer, take, inner, look, shift, put) = (w.name(), w.name(), w.name(), w.name(), w.name(), w.name());
    w.goto(outer, outer);
    let more = w.test(BinOp::Lt, copy(i), copy(n));
    w.branch(more, take, ret, take);
    let x = w.fresh(e.clone(), Rvalue::Use(Operand::Copy(element_at(v, i))));
    let j = w.usize(Rvalue::Use(copy(i)));
    w.goto(inner, inner);
    let above = w.test(BinOp::Gt, copy(j), usize_const(0));
    w.branch(above, look, put, look);
    let below = w.usize(Rvalue::BinaryOp(BinOp::Sub, copy(j), usize_const(1)));
    let y = w.fresh(e, Rvalue::Use(Operand::Copy(element_at(v, below))));
    let less = w.test(BinOp::Lt, copy(x), copy(y));
    w.branch(less, shift, put, shift);
    w.set(element_at(v, j), Rvalue::Use(copy(y)));
    w.set(Place::local(j), Rvalue::Use(copy(below)));
    w.goto(inner, put);
    w.set(element_at(v, j), Rvalue::Use(copy(x)));
    w.step(i);
    w.last(Terminator::Goto(outer));
    Some(())
}

/// `binary_search(&x)`, which rustc calls as `binary_search_by` with core's closure `|p| p.cmp(x)`: core's search,
/// which halves what is left until one element is, never stopping early, so that it finds the element core finds
/// among equal ones and on a slice that isn't sorted. Its halving is a loop of subtractions, as a card has no
/// divide.
fn binary_search(
    w: &mut Writer,
    types: &HashMap<Local, Ty>,
    x: Operand,
    args: &[Operand],
    dest: &Place,
    result: &Ty,
    ret: BasicBlock,
) -> Option<()> {
    let [s, _] = args else { return None };
    let (s, e, _) = slice_arg(types, s)?;
    let Ty::Path(p) = result else { return None };
    let variant = |v: &str, o: Operand| {
        let mut p = p.clone();
        p.segments.push(Segment::Name {
            name: v.into(),
            args: None,
        });
        Rvalue::Variant(p, vec![o])
    };
    let x = w.fresh(e.clone(), Rvalue::Use(x));
    let base = w.usize(Rvalue::Use(usize_const(0)));
    match types.get(&s).and_then(array_len) {
        Some(0) => {
            w.set(dest.clone(), variant("Err", usize_const(0)));
            w.last(Terminator::Goto(ret));
            return Some(());
        }
        // a length known here: each halving is known too, and the search is the steps written out
        Some(mut size) => {
            while size > 1 {
                let half = size / 2;
                let mid = w.usize(Rvalue::BinaryOp(BinOp::Add, copy(base), usize_const(half)));
                let y = w.fresh(e.clone(), Rvalue::Use(Operand::Copy(element_at(s, mid))));
                let above = w.test(BinOp::Gt, copy(y), copy(x));
                let [raise, next] = [(); 2].map(|_| w.name());
                w.branch(above, next, raise, raise);
                w.set(Place::local(base), Rvalue::Use(copy(mid)));
                w.goto(next, next);
                size -= half;
            }
        }
        None => {
            let [empty, go, head, body, halve, sub, pick, raise, shrink, tail] = [(); 10].map(|_| w.name());
            let size = w.len(s);
            let any = w.test(BinOp::Gt, copy(size), usize_const(0));
            w.branch(any, go, empty, empty);
            w.set(dest.clone(), variant("Err", usize_const(0)));
            w.goto(ret, go);
            w.goto(head, head);
            let more = w.test(BinOp::Gt, copy(size), usize_const(1));
            w.branch(more, body, tail, body);
            // half = size / 2
            let half = w.usize(Rvalue::Use(usize_const(0)));
            let left = w.usize(Rvalue::Use(copy(size)));
            w.goto(halve, halve);
            let two = w.test(BinOp::Ge, copy(left), usize_const(2));
            w.branch(two, sub, pick, sub);
            w.set(
                Place::local(left),
                Rvalue::BinaryOp(BinOp::Sub, copy(left), usize_const(2)),
            );
            w.step(half);
            w.goto(halve, pick);
            // the element half past the base: the base moves up to it unless it is above x
            let mid = w.usize(Rvalue::BinaryOp(BinOp::Add, copy(base), copy(half)));
            let y = w.fresh(e.clone(), Rvalue::Use(Operand::Copy(element_at(s, mid))));
            let above = w.test(BinOp::Gt, copy(y), copy(x));
            w.branch(above, shrink, raise, raise);
            w.set(Place::local(base), Rvalue::Use(copy(mid)));
            w.goto(shrink, shrink);
            w.set(Place::local(size), Rvalue::BinaryOp(BinOp::Sub, copy(size), copy(half)));
            w.goto(head, tail);
        }
    }
    let [found, missed, up, down] = [(); 4].map(|_| w.name());
    // one element left: Ok at x, else Err where x would go
    let y = w.fresh(e, Rvalue::Use(Operand::Copy(element_at(s, base))));
    let equal = w.test(BinOp::Eq, copy(y), copy(x));
    w.branch(equal, found, missed, found);
    w.set(dest.clone(), variant("Ok", copy(base)));
    w.goto(ret, missed);
    let below = w.test(BinOp::Lt, copy(y), copy(x));
    w.branch(below, up, down, up);
    let after = w.usize(Rvalue::BinaryOp(BinOp::Add, copy(base), usize_const(1)));
    w.set(dest.clone(), variant("Err", copy(after)));
    w.goto(ret, down);
    w.set(dest.clone(), variant("Err", copy(base)));
    w.last(Terminator::Goto(ret));
    Some(())
}

/// Each call of one of these methods, in the blocks it becomes; and what was there only for the call (the closure
/// `binary_search` passes, the `&mut` of `<`) goes with it.
pub fn write_calls(f: &mut Function) {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    let mut w = Writer {
        local: types.keys().map(|l| l.0).max().unwrap_or(0) + 1,
        block: f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1,
        decls: vec![],
        blocks: vec![],
        at: BasicBlock(0),
        statements: vec![],
    };
    // a pair split_at made: its two slices, which its fields are
    let mut pairs: HashMap<Local, (Local, Local)> = HashMap::new();
    let mut passed: Vec<Local> = vec![];
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            unwind: Unwind::Continue,
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let n = names(&func);
        let start = w.name();
        w.at = start;
        let (local, block, decls, blocks) = (w.local, w.block, w.decls.len(), w.blocks.len());
        let written = if slice_method(&func, "split_at") {
            let d = dest.as_local().filter(|d| fields_only(f, *d, bi));
            d.and_then(|d| {
                split_at(&mut w, &types, &args, ret)
                    .map(|ab| pairs.insert(d, ab))
                    .map(|_| ())
            })
        } else if slice_method(&func, "copy_from_slice") || n.ends_with(&["slice", "copy_from_slice_impl"]) {
            copy_from_slice(&mut w, &types, &args, ret)
        } else if slice_method(&func, "rotate_left") {
            rotate_left(&mut w, &types, &args, ret)
        } else if n.ends_with(&["smallsort", "insertion_sort_shift_left"]) {
            insertion_sort(&mut w, &types, &func, &args, ret)
        } else if slice_method(&func, "binary_search_by") && searches(&func) {
            let x = args.get(1).and_then(|c| captured(f, &types, bi, c));
            let result = dest.as_local().and_then(|d| types.get(&d));
            match (x, result) {
                (Some((x, si)), Some(r)) => binary_search(&mut w, &types, x, &args, &dest, r, ret).map(|()| {
                    // the closure is built only to be passed
                    f.body.blocks[bi].statements.remove(si);
                }),
                _ => None,
            }
        } else {
            continue;
        };
        match written {
            Some(()) => {
                f.body.blocks[bi].terminator = Terminator::Goto(start);
                passed.extend(args.iter().filter_map(Operand::local));
            }
            None => {
                // nothing written after all
                w.local = local;
                w.block = block;
                w.decls.truncate(decls);
                w.blocks.truncate(blocks);
                w.statements.clear();
            }
        }
    }
    f.body.decls.append(&mut w.decls);
    f.body.blocks.append(&mut w.blocks);
    for (d, (a, b)) in &pairs {
        let field = |p: &mut Place| {
            if p.base_local() == Some(*d) {
                if let [Projection::Field(k, _)] = p.projection.as_slice() {
                    *p = Place::local(if *k == 0 { *a } else { *b });
                }
            }
        };
        let mut field = field;
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                s.places_mut(&mut field);
            }
            b.terminator.places_mut(&mut field);
        }
        passed.push(*d);
    }
    unread(f, passed);
}

/// Is d, a call's result in block `at`, read only field by field, `(d.0: T)` and `(d.1: T)`, and set nowhere else?
fn fields_only(f: &Function, d: Local, at: usize) -> bool {
    let mut places = vec![];
    for (bi, b) in f.body.blocks.iter().enumerate() {
        places.extend(b.statements.iter().flat_map(|s| s.places()));
        if bi != at {
            places.extend(b.terminator.places());
        }
    }
    let fields = places
        .iter()
        .filter(|p| p.base_local() == Some(d))
        .all(|p| matches!(p.projection.as_slice(), [Projection::Field(0 | 1, _)]));
    let defined = f.body.blocks.iter().enumerate().any(|(bi, b)| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Assign(p, _) if p.base_local() == Some(d)))
            || (bi != at && b.terminator.def() == Some(d))
    });
    fields && !defined
}

/// Is it `binary_search_by` with the closure `binary_search` passes, `|p| p.cmp(x)`?
fn searches(func: &Path) -> bool {
    let Some(Segment::Name { args: Some(g), .. }) = func.segments.last() else {
        return false;
    };
    g.args
        .iter()
        .any(|a| matches!(a, ir::GenericArg::Ty(Ty::Other(t)) if t.contains("binary_search::{closure")))
}

/// The value `binary_search`'s closure c compares with, `*r` of the `&x` it holds, and where c is built in block
/// `at`: `c = {closure@..} { 0: copy r }`, as the IR reads it, or as the line the closure pass leaves unread.
fn captured(f: &Function, types: &HashMap<Local, Ty>, at: usize, c: &Operand) -> Option<(Operand, usize)> {
    let c = c.local()?;
    let (si, r) = f.body.blocks[at]
        .statements
        .iter()
        .enumerate()
        .rev()
        .find_map(|(si, s)| match s {
            Statement::Unsupported(u) => match u.locals.as_slice() {
                [d, r] if *d == c => Some((si, *r)),
                _ => None,
            },
            Statement::Assign(p, Rvalue::Closure(_, fields)) if p.as_local() == Some(c) => match fields.as_slice() {
                [(_, o)] => o.local().map(|r| (si, r)),
                _ => None,
            },
            _ => None,
        })?;
    matches!(types.get(&r), Some(Ty::Ref { mutable: false, .. })).then_some(())?;
    let x = Operand::Copy(Place {
        base: PlaceBase::Local(r),
        projection: vec![Projection::Deref],
    });
    Some((x, si))
}

/// The locals a written call was passed (and the pairs split_at made), where nothing reads them now: each goes, and
/// its definitions, and what those read in turn when that is all that read it.
fn unread(f: &mut Function, mut locals: Vec<Local>) {
    let params: Vec<Local> = f.params.iter().map(|(l, _)| *l).collect();
    while let Some(l) = locals.pop() {
        if params.contains(&l) || l == Local(0) {
            continue;
        }
        // what a statement reads: all it names but the local it sets whole
        let reads = |s: &Statement| -> Vec<Local> {
            let mut ps = s.places();
            if matches!(s, Statement::Assign(p, _) if p.as_local().is_some()) {
                ps.remove(0);
            }
            ps.iter().filter_map(|p| p.base_local()).collect()
        };
        let read = f.body.blocks.iter().any(|b| {
            b.statements.iter().any(|s| reads(s).contains(&l))
                || b.terminator.places().iter().any(|q| q.base_local() == Some(l))
        });
        if read {
            continue;
        }
        for b in f.body.blocks.iter_mut() {
            b.statements.retain(|s| match s {
                Statement::Assign(p, _) if p.as_local() == Some(l) => {
                    locals.extend(reads(s));
                    false
                }
                Statement::StorageLive(m) | Statement::StorageDead(m) => *m != l,
                _ => true,
            });
        }
        f.body.decls.retain(|d| d.local != l);
        f.body.debug.retain(|d| d.named_local().is_none_or(|(_, m)| m != l));
    }
}

/// `core::slice::Chunks<'_, T>` or `Windows<'_, T>`: which, and T.
fn chunked(t: &Ty) -> Option<(bool, Ty)> {
    let Ty::Path(p) = t else { return None };
    let n = names(p);
    let windows = match n.as_slice() {
        [.., "slice", "Chunks"] | ["Chunks"] => false,
        [.., "slice", "Windows"] | ["Windows"] => true,
        _ => return None,
    };
    let Some(Segment::Name { args: Some(g), .. }) = p.segments.last() else {
        return None;
    };
    let e = g.args.iter().find_map(|a| match a {
        ir::GenericArg::Ty(t @ Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::I8 | IntTy::Usize)) => Some(t.clone()),
        _ => None,
    })?;
    Some((windows, e))
}

/// The number a constant of a one-field wrapper holds, `NonZero::<usize>(NonZeroUsizeInner(2_usize is 1..))` or 2.
fn wrapped(o: &Operand) -> Option<Operand> {
    fn inner(c: &Const) -> Option<&Const> {
        match c {
            Const::Int { .. } => Some(c),
            Const::Variant(_, cs) => match cs.as_slice() {
                [c] => inner(c),
                _ => None,
            },
            Const::Adt(_, fs) => match fs.as_slice() {
                [(_, c)] => inner(c),
                _ => None,
            },
            _ => None,
        }
    }
    match o {
        Operand::Constant(c) => match inner(c)? {
            Const::Int { value, .. } => Some(usize_const(u64::try_from(*value).ok()?)),
            _ => None,
        },
        o => Some(o.clone()),
    }
}

/// A Chunks or a Windows built, `Chunks { v: s, chunk_size: k }`: s, and k.
fn built(fields: &[(String, Operand)]) -> Option<(Operand, Operand)> {
    let field = |n: &str| fields.iter().find(|(m, _)| m == n).map(|(_, o)| o);
    let k = field("chunk_size").or_else(|| field("size"))?;
    Some((field("v")?.clone(), wrapped(k)?))
}

/// `for c in t.chunks(k)` and `for w in t.windows(k)`, where rustc calls the iterator's `next()`: the iterator is two
/// locals, the slice still to come and k, and each `next()` the slices core's makes: the first k elements (or what is
/// left) and the rest after them, or for a window the first k and the rest after the first. The `Option<&[T]>` it
/// returns is two locals too, the discriminant and the slice, where it is read only as those. Each piece is a slice
/// of its own, made of a pointer and a length, as `run_time_lengths` lays them out.
pub fn chunks_and_windows(f: &mut Function) {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    let iters: Vec<(Local, bool, Ty)> = f
        .body
        .decls
        .iter()
        .filter_map(|d| chunked(&d.ty).map(|(w, e)| (d.local, w, e)))
        .collect();
    if iters.is_empty() {
        return;
    }
    let mut w = Writer {
        local: types.keys().map(|l| l.0).max().unwrap_or(0) + 1,
        block: f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1,
        decls: vec![],
        blocks: vec![],
        at: BasicBlock(0),
        statements: vec![],
    };
    // each iterator's two locals, the rest of the slice and k
    let mut parts: HashMap<Local, (Local, Local)> = HashMap::new();
    for (it, _, e) in &iters {
        let v = w.local(slice_of(e, false));
        let k = w.local(Ty::Int(IntTy::Usize));
        parts.insert(*it, (v, k));
    }
    let item = |l: Local| matches!(types.get(&l), Some(Ty::Path(p)) if p.name() == "Option" || names(p).last() == Some(&"Option"));
    // the borrows `r = &mut it` each next() is passed, and the Options it returns
    let mut nexts: Vec<(usize, Local, Local)> = vec![]; // (block, iterator, the Option)
    for (bi, b) in f.body.blocks.iter().enumerate() {
        let Terminator::Call { dest, func, args, .. } = &b.terminator else {
            continue;
        };
        let (Some(o), [r]) = (dest.as_local(), args.as_slice()) else {
            continue;
        };
        if func.name() != "next" || !item(o) {
            continue;
        }
        let Some(r) = r.local() else { continue };
        let it = b.statements.iter().find_map(|s| match s {
            Statement::Assign(p, Rvalue::Ref(ir::BorrowKind::Mut, q)) if p.as_local() == Some(r) => q.as_local(),
            _ => None,
        });
        if let Some(it) = it.filter(|it| parts.contains_key(it)) {
            nexts.push((bi, it, o));
        }
    }
    // every use of an iterator must be one of these: built, copied, borrowed for its next(); and every use of an
    // Option its discriminant or its Some's field
    let borrows: Vec<Local> = nexts
        .iter()
        .filter_map(|(bi, _, _)| match &f.body.blocks[*bi].terminator {
            Terminator::Call { args, .. } => args[0].local(),
            _ => None,
        })
        .collect();
    let options: Vec<Local> = nexts.iter().map(|(_, _, o)| *o).collect();
    for b in &f.body.blocks {
        for s in &b.statements {
            let Statement::Assign(d, r) = s else {
                if s.places().iter().any(|p| {
                    p.base_local()
                        .is_some_and(|l| parts.contains_key(&l) || options.contains(&l))
                }) {
                    return;
                }
                continue;
            };
            let iter_def = d.as_local().is_some_and(|l| parts.contains_key(&l))
                && match r {
                    Rvalue::Struct(_, fs) => built(fs).is_some(),
                    Rvalue::Use(o) => o.local().is_some_and(|l| parts.contains_key(&l)),
                    _ => false,
                };
            let borrow = d.as_local().is_some_and(|l| borrows.contains(&l));
            let option_read = match r {
                Rvalue::Discriminant(p) => p.as_local().is_some_and(|l| options.contains(&l)),
                Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => {
                    p.base_local().is_some_and(|l| options.contains(&l))
                        && matches!(p.projection.as_slice(), [Projection::Downcast(v), Projection::Field(0, _)] if v == "Some")
                }
                _ => false,
            };
            if iter_def || borrow || option_read {
                continue;
            }
            let named = s.places().iter().any(|p| {
                p.base_local()
                    .is_some_and(|l| parts.contains_key(&l) || options.contains(&l) || borrows.contains(&l))
            });
            if named {
                return;
            }
        }
        let named = b.terminator.places().iter().any(|p| {
            p.base_local()
                .is_some_and(|l| parts.contains_key(&l) || options.contains(&l) || borrows.contains(&l))
        });
        if named && !nexts.iter().any(|(bi, _, _)| f.body.blocks[*bi].name == b.name) {
            return;
        }
    }
    // each Option's two locals
    let mut items: HashMap<Local, (Local, Local)> = HashMap::new();
    for (_, it, o) in &nexts {
        let e = iters.iter().find(|(l, _, _)| l == it).unwrap().2.clone();
        let tag = w.local(Ty::Int(IntTy::Isize));
        let v = w.local(slice_of(&e, false));
        items.insert(*o, (tag, v));
    }
    let tag_const = |v: i128| {
        Rvalue::Use(Operand::Constant(Const::Int {
            value: v,
            ty: IntTy::Isize,
        }))
    };
    for (bi, it, o) in &nexts {
        let Terminator::Call { target: Some(ret), .. } = f.body.blocks[*bi].terminator else {
            return;
        };
        let (windows, e) = iters
            .iter()
            .find(|(l, _, _)| l == it)
            .map(|(_, w, e)| (*w, e.clone()))
            .unwrap();
        let (v, k) = parts[it];
        let (tag, item) = items[o];
        let start = w.name();
        w.at = start;
        let [none, some] = [(); 2].map(|_| w.name());
        let n = w.len(v);
        // none left: a chunk once the slice is empty, a window once it is shorter than k
        let ended = if windows {
            w.test(BinOp::Gt, copy(k), copy(n))
        } else {
            w.test(BinOp::Eq, copy(n), usize_const(0))
        };
        w.branch(ended, none, some, none);
        w.set(Place::local(tag), tag_const(0));
        w.goto(ret, some);
        let p = w.fresh(
            Ty::Ptr {
                mutable: false,
                to: Box::new(e.clone()),
            },
            Rvalue::Use(copy(v)),
        );
        let m = w.usize(Rvalue::Use(copy(k)));
        if !windows {
            // the chunk is k long, or what is left when that is less
            let [small, cut] = [(); 2].map(|_| w.name());
            let short = w.test(BinOp::Lt, copy(n), copy(k));
            w.branch(short, small, cut, small);
            w.set(Place::local(m), Rvalue::Use(copy(n)));
            w.goto(cut, cut);
        }
        w.set(Place::local(item), from_parts(p, copy(m)));
        // the rest: after the chunk, or after the window's first element
        let step = if windows { usize_const(1) } else { copy(m) };
        let q = w.fresh(
            Ty::Ptr {
                mutable: false,
                to: Box::new(e),
            },
            Rvalue::BinaryOp(BinOp::Add, copy(p), step.clone()),
        );
        let r = w.usize(Rvalue::BinaryOp(BinOp::Sub, copy(n), step));
        w.set(Place::local(v), from_parts(q, copy(r)));
        w.set(Place::local(tag), tag_const(1));
        w.last(Terminator::Goto(ret));
        f.body.blocks[*bi].terminator = Terminator::Goto(start);
    }
    f.body.decls.append(&mut w.decls);
    f.body.blocks.append(&mut w.blocks);
    // the iterators' and the Options' uses, in their parts
    let gone: Vec<Local> = parts.keys().chain(items.keys()).chain(&borrows).copied().collect();
    for b in f.body.blocks.iter_mut() {
        let mut statements = vec![];
        for s in std::mem::take(&mut b.statements) {
            let Statement::Assign(d, r) = &s else {
                if !matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if gone.contains(&l)) {
                    statements.push(s);
                }
                continue;
            };
            if let Some((v, k)) = d.as_local().and_then(|l| parts.get(&l)).copied() {
                match r {
                    Rvalue::Struct(_, fs) => {
                        let (x, n) = built(fs).unwrap();
                        statements.push(Statement::Assign(Place::local(v), Rvalue::Use(x)));
                        statements.push(Statement::Assign(Place::local(k), Rvalue::Use(n)));
                    }
                    Rvalue::Use(o) => {
                        let (u, j) = parts[&o.local().unwrap()];
                        statements.push(Statement::Assign(Place::local(v), Rvalue::Use(copy(u))));
                        statements.push(Statement::Assign(Place::local(k), Rvalue::Use(copy(j))));
                    }
                    _ => unreachable!(),
                }
                continue;
            }
            if d.as_local().is_some_and(|l| borrows.contains(&l)) {
                continue;
            }
            let read = match r {
                Rvalue::Discriminant(p) => p.as_local().and_then(|l| items.get(&l)).map(|(t, _)| *t),
                Rvalue::Use(Operand::Copy(p) | Operand::Move(p)) => {
                    p.base_local().and_then(|l| items.get(&l)).map(|(_, v)| *v)
                }
                _ => None,
            };
            match read {
                Some(x) => statements.push(Statement::Assign(d.clone(), Rvalue::Use(copy(x)))),
                None => statements.push(s),
            }
        }
        b.statements = statements;
    }
    f.body.decls.retain(|d| !gone.contains(&d.local));
    f.body
        .debug
        .retain(|d| d.named_local().is_none_or(|(_, l)| !gone.contains(&l)));
}
