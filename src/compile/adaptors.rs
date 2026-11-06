//! Iterator adaptor chains, as the loops they stand for. An adaptor (`map`, `filter`, `enumerate`, `zip`, `chain`,
//! `skip`, `take`, `rev`) is a struct of core's around the iterator it adapts, and what consumes a chain (`sum`,
//! `fold`, `count`, `max`, ...) is a call of core's code, which isn't here: the chain's own method, or, where rustc
//! inlined that, the fold of the iterator at the chain's root with a closure of core's for each adaptor it went
//! through (`map_fold(f, g)`, `filter_fold(p, g)`), each closure's type naming the closures it wraps. Here the chain
//! is read back from where its structs are built and from those types, and the call is written as one loop over
//! the root: each item taken through the adaptors in turn, as core takes it, and on into the consumer, with the
//! program's closures called by their functions' names. What a struct holds (a range's ends, `skip`'s n) is copied
//! where the struct is built, as the struct copies it. A chain used in any other way, an adaptor not here, or a
//! closure that captures, is left as it is, and refused.
use std::collections::HashMap;

use super::front::place_ty;
use super::ir::{
    self, ArrayLen, BasicBlock, BinOp, BorrowKind, CastKind, Const, GenericArg, IntTy, Local, Operand, Path, Place,
    Projection, Rvalue, Segment, Statement, Terminator, Ty, Unwind,
};
use super::iterators::{
    assign, branch, call, card_range, copy_of, definitions, element_pointer, last_args, names, usize_const, variant,
    At, Fresh,
};
use super::structs::Function;

/// A closure of the program's that the chain calls: its function, what it returns, and, for one that captures, the
/// local its struct is copied into where the chain takes it (each call is given a reference to it, as core's).
#[derive(Clone)]
struct Closure {
    function: String,
    ret: Ty,
    env: Option<Local>,
}

/// Where a chain's items come from.
#[derive(Clone)]
enum Source {
    /// `a..b` of a card's type: its values
    Range { lo: Operand, hi: Operand, ty: IntTy },
    /// a slice iterator into one array: the addresses of its elements from `from` up to `to`
    Slice { array: Place, from: u64, to: u64, elem: Ty },
    /// `once(x)`, an `Option<T>`'s iterator: its value, if it has one
    Option { opt: Operand, ty: Ty },
    /// `repeat(x)`: x, without end
    Repeat { value: Operand, ty: Ty },
    /// `successors(first, f)`: each item, then f of it, until f gives `None`
    Successors { first: Operand, succ: Closure, ty: Ty },
    /// `(a..b).step_by(k)`: the range, and k - 1 (as the step_by pass builds core's struct)
    StepBy {
        lo: Operand,
        hi: Operand,
        ty: IntTy,
        minus_one: Operand,
    },
}

/// A chain: a source, backwards where `rev` (true) reverses it, and the adaptors around it.
#[derive(Clone)]
enum Iter {
    Source(Source, bool),
    Map(Box<Iter>, Closure),
    Filter(Box<Iter>, Closure),
    /// the count it starts from
    Enumerate(Box<Iter>, Operand),
    Skip(Box<Iter>, Operand),
    Take(Box<Iter>, Operand),
    Chain(Box<Iter>, Box<Iter>),
    Zip(Box<Iter>, Box<Iter>),
}

/// An item on its way through the chain.
#[derive(Clone)]
enum Item {
    Value(Operand, Ty),
    /// `&a[i]`, an element's address: the array, the index and the element's type
    Elem(Place, Local, Ty),
    /// `(a, b)`: enumerate's and zip's
    Pair(Box<Item>, Box<Item>),
}

impl Item {
    fn ty(&self) -> Ty {
        match self {
            Item::Value(_, t) => t.clone(),
            Item::Elem(_, _, e) => shared(e.clone()),
            Item::Pair(a, b) => Ty::Tuple(vec![a.ty(), b.ty()]),
        }
    }
}

/// What the items are, worked out before any code is written: values of a type, or the addresses of one array's
/// elements (which a consumer can keep as an index).
#[derive(Clone, PartialEq)]
enum Shape {
    Value(Ty),
    Elem(Place, Ty),
    Pair(Box<Shape>, Box<Shape>),
    /// a chain of two sources whose items differ in kind
    Mixed,
}

/// What consumes the chain.
enum Consumer {
    /// `sum`, a fold from `init` with core's add
    Sum(Operand),
    /// `count`, a fold from `init` with core's `+ 1`
    Count(Operand),
    Fold(Operand, Closure),
    ForEach(Closure),
    /// `max` (true) or `min` (false), a `reduce` with core's closure
    Extreme(bool),
}

/// An adaptor a closure of core's adds in front of the consumer it wraps.
enum Stage {
    Map(Closure),
    Filter(Closure),
    Enumerate(Operand),
}

fn shared(t: Ty) -> Ty {
    Ty::Ref {
        lifetime: None,
        mutable: false,
        to: Box::new(t),
    }
}

fn int(value: i128, ty: IntTy) -> Operand {
    Operand::Constant(Const::Int { value, ty })
}

fn binary(op: BinOp, a: Operand, b: Operand) -> Rvalue {
    Rvalue::BinaryOp(op, a, b)
}

fn at(array: &Place, i: Local) -> Place {
    let mut e = array.clone();
    e.projection.push(Projection::Index(i));
    e
}

/// `((o as Some).0: T)`.
fn some_value(o: Local, t: &Ty) -> Place {
    Place {
        base: ir::PlaceBase::Local(o),
        projection: vec![Projection::Downcast("Some".into()), Projection::Field(0, t.clone())],
    }
}

/// The type `Option<T>` is of.
fn option_arg(t: &Ty) -> Option<Ty> {
    let Ty::Path(p) = t else { return None };
    match (names(p)?.last(), last_args(p).as_slice()) {
        (Some(&"Option"), [GenericArg::Ty(t)]) => Some(t.clone()),
        _ => None,
    }
}

/// What the program's closures are and return, and the structs of its own (whose names aren't core's adaptors).
pub struct Known<'a> {
    /// each closure that captures nothing: its function, by the closure's type
    pub closures: &'a HashMap<String, String>,
    /// each function's return type
    pub rets: &'a HashMap<String, Ty>,
    /// each closure that captures: its function, by the closure's type
    pub capturing: &'a HashMap<String, String>,
    /// each closure's parameters and body, as the function is before its first parameter goes
    pub bodies: &'a HashMap<String, (Vec<(Local, Ty)>, ir::Body)>,
    pub own_structs: &'a [String],
}

/// The chain one call consumes, read back, and the code it becomes: fresh locals and blocks, and what becomes of
/// each line that built the chain (the copies taken where it is, of what it holds).
struct Gen<'a> {
    f: &'a Function,
    known: &'a Known<'a>,
    defs: HashMap<Local, Vec<At>>,
    types: HashMap<Local, Ty>,
    fresh: Fresh,
    edits: Vec<(At, Vec<Statement>)>,
    held: Vec<Local>,
    /// set when a sink met an item it can't take, which leaves the call as it is
    failed: bool,
}

type Sink<'s, 'a> = dyn FnMut(&mut Gen<'a>, Item, BasicBlock) -> BasicBlock + 's;

impl<'a> Gen<'a> {
    fn local(&mut self, t: Ty) -> Local {
        self.fresh.local(t)
    }

    fn name(&mut self) -> BasicBlock {
        self.fresh.name()
    }

    fn block(&mut self, statements: Vec<Statement>, terminator: Terminator) -> BasicBlock {
        let b = self.fresh.name();
        self.fresh.block(b, statements, terminator);
        b
    }

    fn ty_of(&self, o: &Operand) -> Option<Ty> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => place_ty(&self.types, p),
            Operand::Constant(Const::Int { ty, .. }) => Some(Ty::Int(*ty)),
            Operand::Constant(Const::Bool(_)) => Some(Ty::Bool),
            _ => None,
        }
    }

    /// The value of `o` where the struct holding it is built: a constant, or a copy taken there.
    fn snap(&mut self, o: &Operand, snaps: &mut Vec<Statement>) -> Option<Operand> {
        match o {
            Operand::Constant(c) if c.is_scalar() => Some(o.clone()),
            Operand::Copy(_) | Operand::Move(_) => {
                let t = self.ty_of(o)?;
                let l = self.local(t);
                snaps.push(assign(l, Rvalue::Use(o.clone())));
                Some(copy_of(l))
            }
            _ => None,
        }
    }

    /// A closure of the program's that captures nothing, by its type.
    fn closure_named(&self, text: &str) -> Option<Closure> {
        let function = self.known.closures.get(text)?.clone();
        let ret = self.known.rets.get(&function)?.clone();
        Some(Closure {
            function,
            ret,
            env: None,
        })
    }

    /// The closure of the program's an operand holds: one that captures nothing, as a constant, or one that
    /// captures, its struct (`main_closure0_env`), copied into a local of its own by `snaps`.
    fn closure(&mut self, o: &Operand, snaps: &mut Vec<Statement>) -> Option<Closure> {
        if let Operand::Constant(Const::ZeroSized(Ty::Other(t))) = o {
            return self.closure_named(t);
        }
        let Some(Ty::Path(e)) = self.ty_of(o) else { return None };
        let function = self
            .known
            .capturing
            .values()
            .find(|f| e.is(&format!("{f}_env")))?
            .clone();
        let ret = self.known.rets.get(&function)?.clone();
        let env = self.snap(o, snaps)?.local()?;
        Some(Closure {
            function,
            ret,
            env: Some(env),
        })
    }

    /// The type of the closure an operand holds, as rustc writes it (and a closure of core's names those it wraps):
    /// a constant's, a closure of core's local's, or the type of the program's closure whose struct a local is.
    fn closure_text(&self, o: &Operand) -> Option<String> {
        match o {
            Operand::Constant(Const::ZeroSized(Ty::Other(t)) | Const::Zeroed(Ty::Other(t), _)) => Some(t.clone()),
            _ => match self.ty_of(o)? {
                Ty::Other(t) => Some(t),
                Ty::Path(e) => self
                    .known
                    .capturing
                    .iter()
                    .find(|(_, f)| e.is(&format!("{f}_env")))
                    .map(|(t, _)| t.clone()),
                _ => None,
            },
        }
    }

    /// A block of `lines` and then a call of `c`, its result in `dest`, and on to `then`. A closure that is a few
    /// blocks of plain code (no calls, nothing the IR keeps as text) is its code instead, copied in with locals of
    /// its own: a closure here is called from one place, so the copy costs no cells, and saves the call's.
    fn calling(
        &mut self,
        mut lines: Vec<Statement>,
        c: &Closure,
        args: Vec<Operand>,
        dest: Local,
        then: BasicBlock,
    ) -> BasicBlock {
        let Some((params, body)) = self.known.bodies.get(&c.function) else {
            return self.block(lines, call(Place::local(dest), &c.function, args, then));
        };
        let Some((own, by_mut)) = params
            .first()
            .map(|(l, t)| (*l, matches!(t, Ty::Ref { mutable: true, .. })))
        else {
            return self.block(lines, call(Place::local(dest), &c.function, args, then));
        };
        // the closure's struct, read through its first parameter: `(*_1).k` is `env.k` in a copy
        let through = |p: &Place| p.base_local() != Some(own) || p.projection.first() == Some(&Projection::Deref);
        // (every local the body names declared, so that each has a copy)
        let declared =
            |l: Local| l == Local(0) || params.iter().any(|(p, _)| *p == l) || body.decls.iter().any(|d| d.local == l);
        let inline = leaf(body)
            && params.len() == args.len() + 1
            && body
                .blocks
                .iter()
                .all(|b| b.lines().all(|x| x.locals().into_iter().all(declared)))
            && match c.env {
                None => ir::occurrences(&body.blocks, own) == 0,
                Some(_) => body.blocks.iter().all(|b| {
                    b.statements
                        .iter()
                        .flat_map(|s| s.places())
                        .chain(b.terminator.places())
                        .all(through)
                        && !b.statements.iter().any(
                            |s| matches!(s, Statement::Assign(_, Rvalue::Ref(_, p)) if p.base_local() == Some(own)),
                        )
                }),
            };
        if !inline {
            let mut args = args;
            if let Some(e) = c.env {
                let r = self.local(params[0].1.clone());
                let kind = if by_mut { BorrowKind::Mut } else { BorrowKind::Shared };
                lines.push(assign(r, Rvalue::Ref(kind, Place::local(e))));
                args.insert(0, Operand::Move(Place::local(r)));
            }
            return self.block(lines, call(Place::local(dest), &c.function, args, then));
        }
        let mut locals: HashMap<Local, Local> = HashMap::from([(Local(0), dest)]);
        for ((l, t), a) in params[1..].iter().zip(args) {
            let k = self.local(t.clone());
            locals.insert(*l, k);
            lines.push(assign(k, Rvalue::Use(a)));
        }
        for d in &body.decls {
            if d.local != Local(0) {
                let k = self.local(d.ty.clone());
                locals.insert(d.local, k);
            }
        }
        let names: HashMap<BasicBlock, BasicBlock> = body.blocks.iter().map(|b| (b.name, self.name())).collect();
        let env = c.env;
        let rename = &mut |p: &mut Place| {
            match (p.base_local(), env) {
                (Some(l), Some(e)) if l == own => {
                    p.base = ir::PlaceBase::Local(e);
                    p.projection.remove(0);
                }
                (Some(l), _) => p.base = ir::PlaceBase::Local(locals[&l]),
                _ => {}
            }
            for x in p.projection.iter_mut() {
                if let Projection::Index(i) | Projection::IndexPlus(i, _) = x {
                    *i = locals[i];
                }
            }
        };
        for b in &body.blocks {
            let mut b = b.clone();
            for st in b.statements.iter_mut() {
                st.places_mut(rename);
            }
            b.terminator.places_mut(rename);
            for t in b.terminator.targets_mut() {
                *t = names[t];
            }
            if b.terminator == Terminator::Return {
                b.terminator = Terminator::Goto(then);
            }
            self.fresh.block(names[&b.name], b.statements, b.terminator);
        }
        self.block(lines, Terminator::Goto(names[&body.blocks[0].name]))
    }

    /// The local an iterator is held in and the line that defines it: defined once, and otherwise only read whole
    /// (rustc's MIR reads one value of an iterator again where two chains start alike, `let r = a..b;` used twice),
    /// so that each read is the value it was built with.
    fn held_at(&mut self, o: &Operand) -> Option<(Local, At)> {
        let (Operand::Move(p) | Operand::Copy(p)) = o else {
            return None;
        };
        let l = p.as_local()?;
        let [at] = self.defs.get(&l)?.as_slice() else {
            return None;
        };
        let blocks = &self.f.body.blocks;
        let whole = |o: &&Operand| matches!(*o, Operand::Copy(q) | Operand::Move(q) if q.as_local() == Some(l));
        let reads: usize = blocks
            .iter()
            .map(|b| {
                let lines = b
                    .statements
                    .iter()
                    .map(|s| s.operands().into_iter().filter(whole).count())
                    .sum::<usize>();
                lines + b.terminator.operands().into_iter().filter(whole).count()
            })
            .sum();
        if ir::occurrences(blocks, l) != reads + 1 {
            return None;
        }
        self.held.push(l);
        Some((l, *at))
    }

    /// The chain an operand holds, from where its structs are built.
    fn iter_of(&mut self, o: &Operand) -> Option<Iter> {
        // a range rustc made a constant of, `const core::ops::Range::<i16> {{ start: 0_i16, end: 2_i16 }}`
        if let Operand::Constant(Const::Adt(p, fs)) = o {
            let ty = card_range(&Ty::Path(p.clone()))?;
            let [(st, lo @ Const::Int { .. }), (en, hi @ Const::Int { .. })] = fs.as_slice() else {
                return None;
            };
            return (st == "start" && en == "end").then(|| {
                let (lo, hi) = (Operand::Constant(lo.clone()), Operand::Constant(hi.clone()));
                Iter::Source(Source::Range { lo, hi, ty }, false)
            });
        }
        let (_, at) = self.held_at(o)?;
        let f = self.f;
        let mut snaps = vec![];
        let it = match at {
            At::Line(bi, si) => {
                let Statement::Assign(_, r) = &f.body.blocks[bi].statements[si] else {
                    return None;
                };
                self.built(r, &mut snaps)?
            }
            At::Call(bi) => {
                let Terminator::Call { func, args, .. } = &f.body.blocks[bi].terminator else {
                    return None;
                };
                self.called(func, args, &mut snaps)?
            }
        };
        self.edits.push((at, snaps));
        Some(it)
    }

    /// An iterator built by a statement: a copy of another, a range, a slice iterator, or a struct of core's.
    fn built(&mut self, r: &Rvalue, snaps: &mut Vec<Statement>) -> Option<Iter> {
        let (p, fs) = match r {
            Rvalue::Use(o @ (Operand::Copy(_) | Operand::Move(_))) => return self.iter_of(o),
            Rvalue::Struct(p, fs) => (p, fs),
            _ => return None,
        };
        let fields: Vec<&str> = fs.iter().map(|(n, _)| n.as_str()).collect();
        let op = |k: usize| &fs[k].1;
        let source = |s: Source| Some(Iter::Source(s, false));
        if let Some(ty) = card_range(&Ty::Path(p.clone())) {
            if fields != ["start", "end"] {
                return None;
            }
            let (lo, hi) = (self.snap(op(0), snaps)?, self.snap(op(1), snaps)?);
            return source(Source::Range { lo, hi, ty });
        }
        let name = p.name();
        if p.is_plain() && name.starts_with("slice_Iter") {
            let [(_, ptr), (_, end)] = fs.as_slice() else {
                return None;
            };
            let (array, from) = element_pointer(self.f, &self.defs, ptr)?;
            let (other, to) = element_pointer(self.f, &self.defs, end)?;
            let Some(Ty::Array(elem, _) | Ty::Slice(elem)) = place_ty(&self.types, &array) else {
                return None;
            };
            return (array == other && from <= to).then(|| {
                Iter::Source(
                    Source::Slice {
                        array,
                        from,
                        to,
                        elem: *elem,
                    },
                    false,
                )
            });
        }
        // `StepBy_i16 { iter, step_minus_one, first_take: true }`, as the step_by pass builds it
        if p.is_plain() && name.starts_with("StepBy_") {
            let (["iter", "step_minus_one", "first_take"], Operand::Constant(Const::Bool(true))) =
                (fields.as_slice(), op(2))
            else {
                return None;
            };
            let Iter::Source(Source::Range { lo, hi, ty }, false) = self.iter_of(op(0))? else {
                return None;
            };
            let minus_one = self.snap(op(1), snaps)?;
            return source(Source::StepBy { lo, hi, ty, minus_one });
        }
        // core's structs are generic; one of the program's own of the same name isn't taken for one
        if p.qself.is_some() || last_args(p).is_empty() || self.known.own_structs.iter().any(|s| s == name) {
            return None;
        }
        Some(match (name, fields.as_slice()) {
            ("Map", ["iter", "f"]) => Iter::Map(Box::new(self.iter_of(op(0))?), self.closure(op(1), snaps)?),
            ("Filter", ["iter", "predicate"]) => {
                Iter::Filter(Box::new(self.iter_of(op(0))?), self.closure(op(1), snaps)?)
            }
            ("Skip", ["iter", "n"]) => Iter::Skip(Box::new(self.iter_of(op(0))?), self.snap(op(1), snaps)?),
            ("Take", ["iter", "n"]) => Iter::Take(Box::new(self.iter_of(op(0))?), self.snap(op(1), snaps)?),
            ("Enumerate", ["iter", "count"]) => {
                Iter::Enumerate(Box::new(self.iter_of(op(0))?), self.snap(op(1), snaps)?)
            }
            ("Rev", ["iter"]) => reversed(self.iter_of(op(0))?)?,
            ("Repeat", ["element"]) => {
                let ty = self.ty_of(op(0))?;
                Iter::Source(
                    Source::Repeat {
                        value: self.snap(op(0), snaps)?,
                        ty,
                    },
                    false,
                )
            }
            // `Once { inner: IntoIter { inner: Item { opt } } }`
            ("Once", ["inner"]) => {
                let opt = self.option_inside(op(0))?;
                let ty = option_arg(&self.ty_of(&opt)?)?;
                Iter::Source(
                    Source::Option {
                        opt: self.snap(&opt, snaps)?,
                        ty,
                    },
                    false,
                )
            }
            ("Successors", ["next", "succ"]) => {
                let ty = option_arg(&self.ty_of(op(0))?)?;
                let succ = self.closure(op(1), snaps)?;
                Iter::Source(
                    Source::Successors {
                        first: self.snap(op(0), snaps)?,
                        succ,
                        ty,
                    },
                    false,
                )
            }
            _ => return None,
        })
    }

    /// The `Option` an `option::IntoIter` holds, through the structs core wraps it in.
    fn option_inside(&mut self, o: &Operand) -> Option<Operand> {
        let (_, At::Line(bi, si)) = self.held_at(o)? else {
            return None;
        };
        let Statement::Assign(_, Rvalue::Struct(p, fs)) = &self.f.body.blocks[bi].statements[si] else {
            return None;
        };
        let [(field, inner)] = fs.as_slice() else { return None };
        let found = match (p.name(), field.as_str()) {
            ("IntoIter", "inner") => self.option_inside(inner)?,
            ("Item", "opt") => inner.clone(),
            _ => return None,
        };
        self.edits.push((At::Line(bi, si), vec![]));
        Some(found)
    }

    /// An iterator a call of an adaptor's method returns (where rustc doesn't inline the method).
    fn called(&mut self, func: &Path, args: &[Operand], snaps: &mut Vec<Statement>) -> Option<Iter> {
        let tr = func.qself.as_ref()?.as_trait.as_ref().and_then(names)?;
        if tr != ["Iterator"] && tr != ["IntoIterator"] && tr != ["DoubleEndedIterator"] {
            return None;
        }
        Some(match (func.name(), args) {
            ("chain", [a, b]) => Iter::Chain(Box::new(self.iter_of(a)?), Box::new(self.iter_of(b)?)),
            ("zip", [a, b]) => Iter::Zip(Box::new(self.iter_of(a)?), Box::new(self.iter_of(b)?)),
            ("map", [a, c]) => Iter::Map(Box::new(self.iter_of(a)?), self.closure(c, snaps)?),
            ("filter", [a, c]) => Iter::Filter(Box::new(self.iter_of(a)?), self.closure(c, snaps)?),
            ("enumerate", [a]) => Iter::Enumerate(Box::new(self.iter_of(a)?), usize_const(0)),
            ("skip", [a, n]) => Iter::Skip(Box::new(self.iter_of(a)?), self.snap(n, snaps)?),
            ("take", [a, n]) => Iter::Take(Box::new(self.iter_of(a)?), self.snap(n, snaps)?),
            ("rev", [a]) => reversed(self.iter_of(a)?)?,
            ("into_iter", [a]) => self.iter_of(a)?,
            _ => return None,
        })
    }

    /// The closure a fold is given: `o`, of type `text` (or, inside a closure of core's with no bytes of its own,
    /// no operand). The program's own is the fold's; one of core's is read from its type, the adaptors it stands
    /// for and the consumer it wraps. `zeroed` says the value is all zero bytes (enumerate's count is 0, and every
    /// closure it wraps captures nothing). A closure of core's that holds something is built where the chain is,
    /// `{closure@..map_fold..} { 0: g, 1: f }`: the program's closure among its fields is found by its type (which
    /// names no path), and the one it goes on to is the other.
    fn fold_closure(
        &mut self,
        text: &str,
        o: Option<&Operand>,
        zeroed: bool,
        init: &Operand,
        snaps: &mut Vec<Statement>,
    ) -> Option<(Vec<Stage>, Consumer)> {
        if let Some(c) = self.closure_named(text) {
            return Some((vec![], Consumer::Fold(init.clone(), c)));
        }
        if self.known.capturing.contains_key(text) {
            let c = self.closure(o?, snaps)?;
            return Some((vec![], Consumer::Fold(init.clone(), c)));
        }
        // its value, where one was built (the type it is built with names core's paths in full)
        let (text, fields, built) = match o {
            Some(o @ (Operand::Copy(_) | Operand::Move(_))) => {
                let (_, at @ At::Line(bi, si)) = self.held_at(o)? else {
                    return None;
                };
                let Statement::Assign(_, Rvalue::Closure(Ty::Other(t), fs)) = &self.f.body.blocks[bi].statements[si]
                else {
                    return None;
                };
                (t.clone(), Some(fs.clone()), Some(at))
            }
            _ => (text.to_string(), None, None),
        };
        let Ok(Ty::Path(p)) = ir::parse_ty(text.strip_prefix("{closure@")?.strip_suffix("::{closure#0}}")?) else {
            return None;
        };
        let n = names(&p)?;
        let args: Vec<&Ty> = last_args(&p)
            .into_iter()
            .filter_map(|a| match a {
                GenericArg::Ty(t) => Some(t),
                _ => None,
            })
            .collect();
        let other = |t: &Ty| match t {
            Ty::Other(t) => Some(t.clone()),
            _ => None,
        };
        let mut lines = vec![];
        // the program's closure of type t, and the field that holds it
        let own = |g: &mut Self, t: &Ty, lines: &mut Vec<Statement>| -> Option<(Closure, Option<usize>)> {
            let t = other(t)?;
            match &fields {
                None => Some((g.closure_named(&t)?, None)),
                Some(fs) => {
                    let k = fs
                        .iter()
                        .position(|(_, v)| g.closure_text(v).as_deref() == Some(t.as_str()))?;
                    Some((g.closure(&fs[k].1, lines)?, Some(k)))
                }
            }
        };
        let tr = p.qself.as_ref().and_then(|q| q.as_trait.as_ref()).and_then(names);
        let tr = tr.as_ref().and_then(|t| t.last().copied());
        let (stage, rest, used) = match (n.as_slice(), args.as_slice(), tr) {
            ([.., "map", "map_fold"], [_, _, _, f, g], None) => {
                let (c, k) = own(self, f, &mut lines)?;
                (Stage::Map(c), *g, k)
            }
            ([.., "filter", "filter_fold"], [_, _, p, g], None) => {
                let (c, k) = own(self, p, &mut lines)?;
                (Stage::Filter(c), *g, k)
            }
            (["fold", "enumerate"], [_, _, g], Some("Iterator")) => {
                let q = &p.qself.as_ref()?.ty;
                matches!(q, Ty::Path(e) if e.name() == "Enumerate").then_some(())?;
                let (count, k) = match &fields {
                    Some(fs) => {
                        let k = fs
                            .iter()
                            .position(|(_, v)| self.ty_of(v) == Some(Ty::Int(IntTy::Usize)))?;
                        (self.snap(&fs[k].1.clone(), &mut lines)?, Some(k))
                    }
                    None if zeroed => (usize_const(0), None),
                    None => return None,
                };
                (Stage::Enumerate(count), *g, k)
            }
            (["sum"], _, Some("Sum")) => return Some((vec![], Consumer::Sum(init.clone()))),
            (["count"], [], Some("Iterator")) => return Some((vec![], Consumer::Count(init.clone()))),
            // `for_each(f)`: a fold with a closure that calls f on each item
            ([.., "for_each", "call"], [_, f], None) => {
                let (c, _) = own(self, f, &mut lines)?;
                match built {
                    Some(at) => self.edits.push((at, lines)),
                    None => snaps.extend(lines),
                }
                return Some((vec![], Consumer::ForEach(c)));
            }
            _ => return None,
        };
        // (what it holds is copied where it is built, as it copies it)
        match built {
            Some(at) => self.edits.push((at, lines)),
            None => snaps.extend(lines),
        }
        // the closure it goes on to: the other of its two fields
        let next = match (&fields, used) {
            (None, _) => None,
            (Some(fs), Some(k)) if fs.len() == 2 => Some(fs[1 - k].1.clone()),
            _ => return None,
        };
        let t = match &next {
            Some(v) => self.closure_text(v)?,
            None => other(rest)?,
        };
        let (mut stages, c) = self.fold_closure(&t, next.as_ref(), zeroed, init, snaps)?;
        stages.insert(0, stage);
        Some((stages, c))
    }

    /// The items' shape, if the chain is one this pass writes.
    fn shape(&self, it: &Iter) -> Option<Shape> {
        Some(match it {
            Iter::Source(Source::Range { ty, .. } | Source::StepBy { ty, .. }, _) => Shape::Value(Ty::Int(*ty)),
            Iter::Source(Source::Slice { array, elem, .. }, _) => Shape::Elem(array.clone(), elem.clone()),
            Iter::Source(Source::Option { ty, .. } | Source::Repeat { ty, .. } | Source::Successors { ty, .. }, _) => {
                Shape::Value(ty.clone())
            }
            Iter::Map(i, c) => {
                self.shape(i)?;
                Shape::Value(c.ret.clone())
            }
            Iter::Filter(i, _) | Iter::Skip(i, _) | Iter::Take(i, _) => self.shape(i)?,
            Iter::Enumerate(i, _) => {
                Shape::Pair(Box::new(Shape::Value(Ty::Int(IntTy::Usize))), Box::new(self.shape(i)?))
            }
            Iter::Chain(a, b) => {
                let (a, b) = (self.shape(a)?, self.shape(b)?);
                if a == b {
                    a
                } else {
                    Shape::Mixed
                }
            }
            // zip only of two sources with no closures of their own, so that which items each gives up past the
            // shorter's end can't be seen
            Iter::Zip(a, b) => {
                let plain = |i: &Iter| matches!(i, Iter::Source(s, _) if !matches!(s, Source::Successors { .. }));
                let pulled = matches!(**b, Iter::Source(Source::Range { .. } | Source::Slice { .. }, _));
                if !plain(a) || !pulled {
                    return None;
                }
                Shape::Pair(Box::new(self.shape(a)?), Box::new(self.shape(b)?))
            }
        })
    }

    /// The statements that make an item one operand, and the operand.
    fn materialize(&mut self, item: &Item) -> (Vec<Statement>, Operand) {
        match item {
            Item::Value(o, _) => (vec![], o.clone()),
            Item::Elem(a, i, e) => {
                let x = self.local(shared(e.clone()));
                (vec![assign(x, Rvalue::Ref(BorrowKind::Shared, at(a, *i)))], copy_of(x))
            }
            Item::Pair(p, q) => {
                let (mut lines, a) = self.materialize(p);
                let (more, b) = self.materialize(q);
                lines.extend(more);
                let t = self.local(item.ty());
                lines.push(assign(t, Rvalue::Tuple(vec![a, b])));
                (lines, copy_of(t))
            }
        }
    }

    /// The item in a local of its own, for a closure that takes a reference to it: the one an address or a pair is
    /// made in, or a copy of a value.
    fn in_local(&mut self, item: &Item) -> (Vec<Statement>, Local) {
        let (mut lines, o) = self.materialize(item);
        match (item, o.local()) {
            (Item::Elem(..) | Item::Pair(..), Some(x)) => (lines, x),
            _ => {
                let x = self.local(item.ty());
                lines.push(assign(x, Rvalue::Use(o)));
                (lines, x)
            }
        }
    }

    /// An integer item's value, read where it is.
    fn value(&mut self, item: &Item) -> Option<Operand> {
        match item {
            Item::Value(o, Ty::Int(_)) => Some(o.clone()),
            Item::Value(Operand::Copy(p), Ty::Ref { to, .. }) if matches!(**to, Ty::Int(_)) => {
                let mut p = p.clone();
                p.projection.push(Projection::Deref);
                Some(Operand::Copy(p))
            }
            Item::Elem(a, i, Ty::Int(_)) => Some(Operand::Copy(at(a, *i))),
            _ => None,
        }
    }

    /// The blocks that take each item of `it` to `sink` (with the block to go on to for the next), and on to `done`
    /// when there are no more: the first of them.
    fn run(&mut self, it: &Iter, sink: &mut Sink<'_, 'a>, done: BasicBlock) -> BasicBlock {
        match it {
            Iter::Source(s, rev) => self.source(s, *rev, sink, done),
            Iter::Map(inner, c) => {
                let c = c.clone();
                self.run(
                    inner,
                    &mut |g, item, next| {
                        let (lines, arg) = g.materialize(&item);
                        let y = g.local(c.ret.clone());
                        let then = sink(g, Item::Value(copy_of(y), c.ret.clone()), next);
                        g.calling(lines, &c, vec![arg], y, then)
                    },
                    done,
                )
            }
            Iter::Filter(inner, p) => {
                let p = p.clone();
                self.run(
                    inner,
                    &mut |g, item, next| {
                        let (mut lines, x) = g.in_local(&item);
                        let ty = item.ty();
                        let r = g.local(shared(ty.clone()));
                        lines.push(assign(r, Rvalue::Ref(BorrowKind::Shared, Place::local(x))));
                        let kept = match item {
                            Item::Elem(..) => item,
                            _ => Item::Value(copy_of(x), ty),
                        };
                        let pass = sink(g, kept, next);
                        let c = g.local(Ty::Bool);
                        let test = g.block(vec![], branch(c, next, pass));
                        g.calling(lines, &p, vec![Operand::Move(Place::local(r))], c, test)
                    },
                    done,
                )
            }
            Iter::Enumerate(inner, count) => {
                let n = self.local(Ty::Int(IntTy::Usize));
                let first = self.run(
                    inner,
                    &mut |g, item, next| {
                        let k = g.local(Ty::Int(IntTy::Usize));
                        let pair = Item::Pair(Box::new(Item::Value(copy_of(k), Ty::Int(IntTy::Usize))), Box::new(item));
                        let then = sink(g, pair, next);
                        let lines = vec![
                            assign(k, Rvalue::Use(copy_of(n))),
                            assign(n, binary(BinOp::Add, copy_of(n), usize_const(1))),
                        ];
                        g.block(lines, Terminator::Goto(then))
                    },
                    done,
                );
                self.block(vec![assign(n, Rvalue::Use(count.clone()))], Terminator::Goto(first))
            }
            // core's skip(n) takes the first n items from the iterator, then folds the rest
            Iter::Skip(inner, n) => {
                let k = self.local(Ty::Int(IntTy::Usize));
                let first = self.run(
                    inner,
                    &mut |g, item, next| {
                        let pass = sink(g, item, next);
                        let down = g.block(
                            vec![assign(k, binary(BinOp::Sub, copy_of(k), usize_const(1)))],
                            Terminator::Goto(next),
                        );
                        let z = g.local(Ty::Bool);
                        g.block(
                            vec![assign(z, binary(BinOp::Eq, copy_of(k), usize_const(0)))],
                            branch(z, down, pass),
                        )
                    },
                    done,
                );
                self.block(vec![assign(k, Rvalue::Use(n.clone()))], Terminator::Goto(first))
            }
            // core's take(n) takes nothing when n is 0, and stops at the nth item without asking for another
            Iter::Take(inner, n) => {
                let k = self.local(Ty::Int(IntTy::Usize));
                let first = self.run(
                    inner,
                    &mut |g, item, next| {
                        let z = g.local(Ty::Bool);
                        let check = g.name();
                        let then = sink(g, item, check);
                        g.fresh.block(
                            check,
                            vec![assign(z, binary(BinOp::Eq, copy_of(k), usize_const(0)))],
                            branch(z, next, done),
                        );
                        g.block(
                            vec![assign(k, binary(BinOp::Sub, copy_of(k), usize_const(1)))],
                            Terminator::Goto(then),
                        )
                    },
                    done,
                );
                let z = self.local(Ty::Bool);
                self.block(
                    vec![
                        assign(k, Rvalue::Use(n.clone())),
                        assign(z, binary(BinOp::Eq, copy_of(k), usize_const(0))),
                    ],
                    branch(z, first, done),
                )
            }
            Iter::Chain(a, b) => {
                let second = self.run(b, sink, done);
                self.run(a, sink, second)
            }
            // b's next() after each of a's items, as core's zip asks: a's item and b's, or the end at b's
            Iter::Zip(a, b) => {
                let Iter::Source(src, rev) = &**b else { unreachable!() };
                let (lo, hi, ty, elems) = match src {
                    Source::Range { lo, hi, ty } => (lo.clone(), hi.clone(), *ty, None),
                    Source::Slice { array, from, to, elem } => (
                        usize_const(*from),
                        usize_const(*to),
                        IntTy::Usize,
                        Some((array.clone(), elem.clone())),
                    ),
                    _ => unreachable!(),
                };
                let rev = *rev;
                let j = self.local(Ty::Int(ty));
                let first = self.run(
                    a,
                    &mut |g, item, next| {
                        let k = g.local(Ty::Int(ty));
                        let other = match &elems {
                            Some((array, elem)) => Item::Elem(array.clone(), k, elem.clone()),
                            None => Item::Value(copy_of(k), Ty::Int(ty)),
                        };
                        let then = sink(g, Item::Pair(Box::new(item), Box::new(other)), next);
                        let (lines, test) = if rev {
                            (
                                vec![
                                    assign(j, binary(BinOp::Sub, copy_of(j), int(1, ty))),
                                    assign(k, Rvalue::Use(copy_of(j))),
                                ],
                                binary(BinOp::Lt, lo.clone(), copy_of(j)),
                            )
                        } else {
                            (
                                vec![
                                    assign(k, Rvalue::Use(copy_of(j))),
                                    assign(j, binary(BinOp::Add, copy_of(j), int(1, ty))),
                                ],
                                binary(BinOp::Lt, copy_of(j), hi.clone()),
                            )
                        };
                        let have = g.block(lines, Terminator::Goto(then));
                        let c = g.local(Ty::Bool);
                        g.block(vec![assign(c, test)], branch(c, done, have))
                    },
                    done,
                );
                let start = if rev { hi.clone() } else { lo.clone() };
                self.block(vec![assign(j, Rvalue::Use(start))], Terminator::Goto(first))
            }
        }
    }

    fn source(&mut self, s: &Source, rev: bool, sink: &mut Sink<'_, 'a>, done: BasicBlock) -> BasicBlock {
        match s {
            Source::Range { .. } | Source::Slice { .. } => {
                let (lo, hi, ty) = match s {
                    Source::Range { lo, hi, ty } => (lo.clone(), hi.clone(), *ty),
                    Source::Slice { from, to, .. } => (usize_const(*from), usize_const(*to), IntTy::Usize),
                    _ => unreachable!(),
                };
                let i = self.local(Ty::Int(ty));
                let item = match s {
                    Source::Slice { array, elem, .. } => Item::Elem(array.clone(), i, elem.clone()),
                    _ => Item::Value(copy_of(i), Ty::Int(ty)),
                };
                let (head, c) = (self.name(), self.local(Ty::Bool));
                if rev {
                    // down from the end: the test, then the step, then the item
                    let body = sink(self, item, head);
                    let down = self.block(
                        vec![assign(i, binary(BinOp::Sub, copy_of(i), int(1, ty)))],
                        Terminator::Goto(body),
                    );
                    self.fresh.block(
                        head,
                        vec![assign(c, binary(BinOp::Lt, lo, copy_of(i)))],
                        branch(c, done, down),
                    );
                    self.block(vec![assign(i, Rvalue::Use(hi))], Terminator::Goto(head))
                } else {
                    let step = self.name();
                    let body = sink(self, item, step);
                    self.fresh.block(
                        step,
                        vec![assign(i, binary(BinOp::Add, copy_of(i), int(1, ty)))],
                        Terminator::Goto(head),
                    );
                    self.fresh.block(
                        head,
                        vec![assign(c, binary(BinOp::Lt, copy_of(i), hi))],
                        branch(c, done, body),
                    );
                    self.block(vec![assign(i, Rvalue::Use(lo))], Terminator::Goto(head))
                }
            }
            Source::Option { opt, ty } => {
                let (o, d, x) = (
                    self.local(super::iterators::option(ty.clone())),
                    self.local(Ty::Int(IntTy::Isize)),
                    self.local(ty.clone()),
                );
                let body = sink(self, Item::Value(copy_of(x), ty.clone()), done);
                let some = self.block(
                    vec![assign(x, Rvalue::Use(Operand::Copy(some_value(o, ty))))],
                    Terminator::Goto(body),
                );
                self.block(
                    vec![
                        assign(o, Rvalue::Use(opt.clone())),
                        assign(d, Rvalue::Discriminant(Place::local(o))),
                    ],
                    branch(d, done, some),
                )
            }
            Source::Repeat { value, ty } => {
                let head = self.name();
                let body = sink(self, Item::Value(value.clone(), ty.clone()), head);
                self.fresh.block(head, vec![], Terminator::Goto(body));
                head
            }
            // core's successors(): the next item worked out before this one is given
            Source::Successors { first, succ, ty } => {
                let (o, d, x, r) = (
                    self.local(super::iterators::option(ty.clone())),
                    self.local(Ty::Int(IntTy::Isize)),
                    self.local(ty.clone()),
                    self.local(shared(ty.clone())),
                );
                let head = self.name();
                let body = sink(self, Item::Value(copy_of(x), ty.clone()), head);
                let take = self.calling(
                    vec![
                        assign(x, Rvalue::Use(Operand::Copy(some_value(o, ty)))),
                        assign(r, Rvalue::Ref(BorrowKind::Shared, Place::local(x))),
                    ],
                    succ,
                    vec![Operand::Move(Place::local(r))],
                    o,
                    body,
                );
                self.fresh.block(
                    head,
                    vec![assign(d, Rvalue::Discriminant(Place::local(o)))],
                    branch(d, done, take),
                );
                self.block(vec![assign(o, Rvalue::Use(first.clone()))], Terminator::Goto(head))
            }
            // core's StepBy: the start, then the range's nth(k - 1) after each item. Neither the sum nor the distance
            // to the end fits a card in every case, so the test is split by the sign of where the range now starts:
            // from 0 or more the distance fits, from below 0 the sum does
            Source::StepBy { lo, hi, ty, minus_one } => {
                let t = *ty;
                let card = Ty::Int(t);
                let (i, cur, m) = (
                    self.local(card.clone()),
                    self.local(card.clone()),
                    self.local(card.clone()),
                );
                let (head, after) = (self.name(), self.name());
                let body = sink(self, Item::Value(copy_of(i), card.clone()), after);
                let c = self.local(Ty::Bool);
                self.fresh.block(
                    head,
                    vec![assign(c, binary(BinOp::Lt, copy_of(i), hi.clone()))],
                    branch(c, done, body),
                );
                let gap = self.local(card.clone());
                let (gu, c1) = (self.local(Ty::Int(IntTy::Usize)), self.local(Ty::Bool));
                let fits = self.block(
                    vec![assign(i, binary(BinOp::Add, copy_of(cur), copy_of(m)))],
                    Terminator::Goto(body),
                );
                let gap_usize = if t == IntTy::Usize {
                    Rvalue::Use(copy_of(gap))
                } else {
                    Rvalue::Cast(copy_of(gap), Ty::Int(IntTy::Usize), CastKind::IntToInt)
                };
                let from_zero = self.block(
                    vec![
                        assign(gap, binary(BinOp::Sub, hi.clone(), copy_of(cur))),
                        assign(gu, gap_usize),
                        assign(c1, binary(BinOp::Lt, minus_one.clone(), copy_of(gu))),
                    ],
                    branch(c1, done, fits),
                );
                let m_card = if t == IntTy::Usize {
                    Rvalue::Use(minus_one.clone())
                } else {
                    Rvalue::Cast(minus_one.clone(), card.clone(), CastKind::IntToInt)
                };
                let mut lines = vec![assign(cur, binary(BinOp::Add, copy_of(i), int(1, t)))];
                let next = if t == IntTy::I16 {
                    let c2 = self.local(Ty::Bool);
                    let below = self.block(
                        vec![
                            assign(i, binary(BinOp::Add, copy_of(cur), copy_of(m))),
                            assign(c2, binary(BinOp::Lt, copy_of(i), hi.clone())),
                        ],
                        branch(c2, done, body),
                    );
                    let neg = self.local(Ty::Bool);
                    lines.push(assign(neg, binary(BinOp::Lt, copy_of(cur), int(0, t))));
                    branch(neg, from_zero, below)
                } else {
                    Terminator::Goto(from_zero)
                };
                // (m, k - 1 as a card, set where the loop starts: below the distance it fits the range's type)
                self.fresh.block(after, lines, next);
                self.block(
                    vec![assign(i, Rvalue::Use(lo.clone())), assign(m, m_card)],
                    Terminator::Goto(head),
                )
            }
        }
    }

    /// The code `c` becomes over `it`, its result in `dest`, then on to `ret`: its first block.
    fn consume(&mut self, it: &Iter, c: Consumer, dest: &Place, ret: BasicBlock) -> Option<BasicBlock> {
        let shape = self.shape(it)?;
        let dest_ty = place_ty(&self.types, dest)?;
        let set = |v: Rvalue| Statement::Assign(dest.clone(), v);
        let finish = |g: &mut Gen<'a>, lines: Vec<Statement>| g.block(lines, Terminator::Goto(ret));
        Some(match c {
            Consumer::Sum(init) => {
                let Ty::Int(t) = dest_ty else { return None };
                let acc = self.local(Ty::Int(t));
                let done = finish(self, vec![set(Rvalue::Use(Operand::Move(Place::local(acc))))]);
                let first = self.run(
                    it,
                    &mut |g, item, next| {
                        let Some(v) = g.value(&item) else {
                            g.failed = true;
                            return next;
                        };
                        let pair = g.local(Ty::Tuple(vec![Ty::Int(t), Ty::Bool]));
                        let after = g.block(
                            vec![assign(
                                acc,
                                Rvalue::Use(Operand::Move(Place::field(pair, 0, Ty::Int(t)))),
                            )],
                            Terminator::Goto(next),
                        );
                        let x = g.local(Ty::Int(t));
                        g.block(
                            vec![
                                assign(x, Rvalue::Use(v)),
                                assign(pair, binary(BinOp::AddWithOverflow, copy_of(acc), copy_of(x))),
                            ],
                            Terminator::Assert {
                                cond: Operand::Move(Place::field(pair, 1, Ty::Bool)),
                                expected: false,
                                msg: "attempt to compute `{} + {}`, which would overflow".into(),
                                msg_args: vec![copy_of(acc), copy_of(x)],
                                target: after,
                                unwind: Unwind::Continue,
                            },
                        )
                    },
                    done,
                );
                self.block(vec![assign(acc, Rvalue::Use(init))], Terminator::Goto(first))
            }
            Consumer::Count(init) => {
                let n = self.local(Ty::Int(IntTy::Usize));
                let done = finish(self, vec![set(Rvalue::Use(Operand::Move(Place::local(n))))]);
                let first = self.run(
                    it,
                    &mut |g, _, next| {
                        g.block(
                            vec![assign(n, binary(BinOp::Add, copy_of(n), usize_const(1)))],
                            Terminator::Goto(next),
                        )
                    },
                    done,
                );
                self.block(vec![assign(n, Rvalue::Use(init))], Terminator::Goto(first))
            }
            Consumer::Fold(init, f) => {
                let acc = self.local(dest_ty.clone());
                let done = finish(self, vec![set(Rvalue::Use(Operand::Move(Place::local(acc))))]);
                let first = self.run(
                    it,
                    &mut |g, item, next| {
                        let (lines, x) = g.materialize(&item);
                        let r = g.local(dest_ty.clone());
                        let after = g.block(
                            vec![assign(acc, Rvalue::Use(Operand::Move(Place::local(r))))],
                            Terminator::Goto(next),
                        );
                        g.calling(lines, &f, vec![Operand::Move(Place::local(acc)), x], r, after)
                    },
                    done,
                );
                self.block(vec![assign(acc, Rvalue::Use(init))], Terminator::Goto(first))
            }
            Consumer::ForEach(f) => {
                let done = finish(self, vec![set(Rvalue::Use(Operand::Constant(Const::Unit)))]);
                self.run(
                    it,
                    &mut |g, item, next| {
                        let (lines, x) = g.materialize(&item);
                        let u = g.local(Ty::Unit);
                        g.calling(lines, &f, vec![x], u, next)
                    },
                    done,
                )
            }
            Consumer::Extreme(max) => {
                let t = option_arg(&dest_ty)?;
                let seen = self.local(Ty::Bool);
                // the one kept: its value, or the index of the element it is
                let (kept, array) = match &shape {
                    Shape::Value(Ty::Int(v)) if t == Ty::Int(*v) => (self.local(t.clone()), None),
                    Shape::Elem(a, e @ Ty::Int(_)) if t == shared(e.clone()) => {
                        (self.local(Ty::Int(IntTy::Usize)), Some(a.clone()))
                    }
                    _ => return None,
                };
                let found = match &array {
                    Some(a) => {
                        let r = self.local(t.clone());
                        vec![
                            assign(r, Rvalue::Ref(BorrowKind::Shared, at(a, kept))),
                            set(variant(&t, Some(copy_of(r)))),
                        ]
                    }
                    None => vec![set(variant(&t, Some(copy_of(kept))))],
                };
                let (some, none) = (finish(self, found), finish(self, vec![set(variant(&t, None))]));
                let done = self.block(vec![], branch(seen, none, some));
                let first = self.run(
                    it,
                    &mut |g, item, next| {
                        let (now, held) = match (&item, &array) {
                            (Item::Elem(_, i, _), Some(a)) => (copy_of(*i), Operand::Copy(at(a, kept))),
                            (Item::Value(o, _), None) => (o.clone(), copy_of(kept)),
                            _ => {
                                g.failed = true;
                                return next;
                            }
                        };
                        let new_value = match (&item, &array) {
                            (Item::Elem(_, i, _), Some(a)) => Operand::Copy(at(a, *i)),
                            _ => now.clone(),
                        };
                        let keep = g.block(vec![assign(kept, Rvalue::Use(now.clone()))], Terminator::Goto(next));
                        let first = g.block(
                            vec![assign(seen, Rvalue::Use(Operand::Constant(Const::Bool(true))))],
                            Terminator::Goto(keep),
                        );
                        // max keeps the new one unless the one kept is above it (the later of equals, as max_by);
                        // min only if the one kept is above it (the earlier)
                        let (x, y, above) = (g.local(t_of(&item)), g.local(t_of(&item)), g.local(Ty::Bool));
                        let compare = g.block(
                            vec![
                                assign(x, Rvalue::Use(held)),
                                assign(y, Rvalue::Use(new_value)),
                                assign(above, binary(BinOp::Gt, copy_of(x), copy_of(y))),
                            ],
                            if max {
                                branch(above, keep, next)
                            } else {
                                branch(above, next, keep)
                            },
                        );
                        g.block(vec![], branch(seen, first, compare))
                    },
                    done,
                );
                self.block(
                    vec![assign(seen, Rvalue::Use(Operand::Constant(Const::Bool(false))))],
                    Terminator::Goto(first),
                )
            }
        })
    }
}

/// Is a closure's body plain code a copy of can stand for the call: assignments, tests, `assert`s and the way back,
/// each place a local's with no index read from a place, and no line the IR keeps as text?
fn leaf(body: &ir::Body) -> bool {
    let plain =
        |p: &Place| p.base_local().is_some() && !p.projection.iter().any(|x| matches!(x, Projection::IndexPlace(_)));
    let operand = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => plain(p),
        Operand::Constant(_) => true,
        Operand::Aggregate(..) | Operand::Fn(_) => false,
    };
    body.blocks.iter().all(|b| {
        let statements = b.statements.iter().all(|s| match s {
            Statement::Assign(p, r) => {
                let places = match r {
                    Rvalue::Ref(_, q) | Rvalue::Discriminant(q) => plain(q),
                    r => r.operands().into_iter().all(operand),
                };
                plain(p) && places
            }
            Statement::Nop => true,
            _ => false,
        });
        let terminator = match &b.terminator {
            Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable => true,
            Terminator::SwitchInt { discr, .. } => operand(discr),
            Terminator::Assert {
                cond, msg_args, unwind, ..
            } => !matches!(unwind, Unwind::Cleanup(_)) && operand(cond) && msg_args.iter().all(operand),
            _ => false,
        };
        statements && terminator
    })
}

/// The value type of an item a `max` or `min` compares.
fn t_of(item: &Item) -> Ty {
    match item {
        Item::Elem(_, _, e) => e.clone(),
        i => i.ty(),
    }
}

/// A chain backwards, as core's `rev()` and `rfold` take it: a source from its end, each adaptor around the
/// reversed iterator, and a chain's two the other way round. Those whose way back needs a length (enumerate, skip,
/// take, zip) or that have none (successors, step_by) aren't reversed here.
fn reversed(it: Iter) -> Option<Iter> {
    Some(match it {
        Iter::Source(s @ (Source::Range { .. } | Source::Slice { .. }), rev) => Iter::Source(s, !rev),
        Iter::Source(s @ (Source::Option { .. } | Source::Repeat { .. }), rev) => Iter::Source(s, rev),
        Iter::Map(i, c) => Iter::Map(Box::new(reversed(*i)?), c),
        Iter::Filter(i, c) => Iter::Filter(Box::new(reversed(*i)?), c),
        Iter::Chain(a, b) => Iter::Chain(Box::new(reversed(*b)?), Box::new(reversed(*a)?)),
        _ => return None,
    })
}

/// `max` (true) or `min` (false) by the closure core's `reduce` is given:
/// `{closure@core::iter::Iterator::max_by::fold<i16, .. {<i16 as Ord>::cmp}>::{closure#0}}`, of an integer or a
/// reference to one.
fn extreme(cl: &Operand) -> Option<bool> {
    let Operand::Constant(Const::ZeroSized(Ty::Other(text))) = cl else {
        return None;
    };
    let Ok(Ty::Path(p)) = ir::parse_ty(text.strip_prefix("{closure@")?.strip_suffix("::{closure#0}}")?) else {
        return None;
    };
    let n = names(&p)?;
    let max = match n.as_slice() {
        [.., "Iterator", "max_by", "fold"] => true,
        [.., "Iterator", "min_by", "fold"] => false,
        _ => return None,
    };
    let [GenericArg::Ty(t), GenericArg::Ty(Ty::Other(cmp))] = last_args(&p).as_slice() else {
        return None;
    };
    let int = match t {
        Ty::Int(_) => true,
        Ty::Ref { to, .. } => matches!(**to, Ty::Int(_)),
        _ => false,
    };
    (int && cmp.ends_with(&format!("{{<{t} as Ord>::cmp}}"))).then_some(max)
}

/// `t.map(f)` on an array, `f` a closure of the program's that captures nothing: rustc's call of core's
/// `try_map` with the closure wrapped to never stop (`NeverShortCircuit::wrap_mut_1(f)`), whose result is the array
/// inside a `NeverShortCircuit`, or of `map` itself. Core calls f on the elements in order; so does the loop it is
/// here, each result into its element of the new array, which the `NeverShortCircuit` local now is.
pub fn array_maps(f: &mut Function, known: &Known) {
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let (Some(d), [array, Operand::Constant(Const::ZeroSized(Ty::Other(closure)))]) =
            (dest.as_local(), args.as_slice())
        else {
            continue;
        };
        let (Some(Segment::Name { name, .. }), Some(Segment::Impl(Ty::Array(t, ArrayLen::Known(n))))) =
            (func.segments.last(), func.segments.get(1))
        else {
            continue;
        };
        if func.qself.is_some()
            || func.segments.len() != 3
            || !matches!(func.segments.first(), Some(Segment::Name { name, .. }) if name == "array")
        {
            continue;
        }
        let (n, t) = (*n, (**t).clone());
        let text = match name.as_str() {
            "map" => closure.clone(),
            // `{closure@ops::try_trait::NeverShortCircuit<T>::wrap_mut_1<T, F>::{closure#0}}`
            "try_map" => {
                let Some(Ok(Ty::Path(p))) = closure
                    .strip_prefix("{closure@")
                    .and_then(|c| c.strip_suffix("::{closure#0}}"))
                    .map(ir::parse_ty)
                else {
                    continue;
                };
                let (Some(w), [GenericArg::Ty(_), GenericArg::Ty(Ty::Other(inner))]) =
                    (names(&p), last_args(&p).as_slice())
                else {
                    continue;
                };
                if !w.ends_with(&["NeverShortCircuit", "wrap_mut_1"]) {
                    continue;
                }
                inner.clone()
            }
            _ => continue,
        };
        let types = super::front::local_types(&f.params, &f.body.decls);
        let mut g = Gen {
            f,
            known,
            defs: HashMap::new(),
            types,
            fresh: Fresh::of(f),
            edits: vec![],
            held: vec![],
            failed: false,
        };
        let Some(c) = g.closure_named(&text) else { continue };
        let Some(src) = array.place().cloned() else { continue };
        // the result: `[U; N]` for `map`, inside the `NeverShortCircuit` for `try_map`
        let result = Ty::Array(Box::new(c.ret.clone()), ArrayLen::Known(n));
        let wrapped = name == "try_map";
        let (i, x, y, c_) = (
            g.local(Ty::Int(IntTy::Usize)),
            g.local(t.clone()),
            g.local(c.ret.clone()),
            g.local(Ty::Bool),
        );
        let (head, step) = (g.name(), g.name());
        let store = g.block(
            vec![Statement::Assign(at(&Place::local(d), i), Rvalue::Use(copy_of(y)))],
            Terminator::Goto(step),
        );
        let body = g.calling(
            vec![assign(x, Rvalue::Use(Operand::Copy(at(&src, i))))],
            &c,
            vec![copy_of(x)],
            y,
            store,
        );
        g.fresh.block(
            step,
            vec![assign(i, binary(BinOp::Add, copy_of(i), usize_const(1)))],
            Terminator::Goto(head),
        );
        g.fresh.block(
            head,
            vec![assign(c_, binary(BinOp::Lt, copy_of(i), usize_const(n)))],
            branch(c_, ret, body),
        );
        let Gen { fresh, .. } = g;
        apply(f, vec![], fresh);
        let b = &mut f.body.blocks[bi];
        b.statements.push(assign(i, Rvalue::Use(usize_const(0))));
        b.terminator = Terminator::Goto(head);
        // the array the `NeverShortCircuit` holds is the local itself
        if let Some(decl) = f.body.decls.iter_mut().find(|x| x.local == d) {
            decl.ty = result.clone();
        }
        if wrapped {
            for b in f.body.blocks.iter_mut() {
                let unwrap = &mut |p: &mut Place| {
                    if p.base_local() == Some(d) && matches!(p.projection.first(), Some(Projection::Field(0, _))) {
                        p.projection.remove(0);
                    }
                };
                for st in b.statements.iter_mut() {
                    st.places_mut(unwrap);
                }
                b.terminator.places_mut(unwrap);
            }
        }
    }
}

/// The chains whose consumer rustc calls, as the loops they are.
pub fn adaptor_chains(f: &mut Function, known: &Known) {
    let mut held: Vec<Local> = vec![];
    for bi in 0..f.body.blocks.len() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            ..
        } = f.body.blocks[bi].terminator.clone()
        else {
            continue;
        };
        let Some(q) = &func.qself else { continue };
        let Some(tr) = q.as_trait.as_ref().and_then(names) else {
            continue;
        };
        let traits: [&[&str]; 6] = [
            &["Iterator"],
            &["DoubleEndedIterator"],
            &["iter", "adapters", "take", "SpecTake"],
            &["iter", "adapters", "step_by", "StepByImpl"],
            &["iter", "adapters", "zip", "ZipImpl"],
            &["iter", "adapters", "zip", "SpecFold"],
        ];
        if !traits.iter().any(|t| tr.as_slice() == *t) {
            continue;
        }
        let Some(recv) = args.first() else { continue };
        let mut g = Gen {
            f,
            known,
            defs: definitions(f),
            types: super::front::local_types(&f.params, &f.body.decls),
            fresh: Fresh::of(f),
            edits: vec![],
            held: vec![],
            failed: false,
        };
        let Some(mut it) = g.iter_of(recv) else { continue };
        let rest = &args[1..];
        let method = func.name();
        // copies of closures the call is given, taken where it is
        let mut at_call = vec![];
        let consumer = match (method, rest) {
            ("fold" | "spec_fold" | "rfold", [init, cl]) => {
                if method == "rfold" {
                    let Some(r) = reversed(it) else { continue };
                    it = r;
                }
                let zeroed = matches!(cl, Operand::Constant(Const::Zeroed(..)));
                let read = match g.closure_text(cl) {
                    Some(t) => g.fold_closure(&t, Some(cl), zeroed, init, &mut at_call),
                    None => None,
                };
                let Some((stages, c)) = read else { continue };
                for s in stages {
                    it = match s {
                        Stage::Map(c) => Iter::Map(Box::new(it), c),
                        Stage::Filter(c) => Iter::Filter(Box::new(it), c),
                        Stage::Enumerate(n) => Iter::Enumerate(Box::new(it), n),
                    };
                }
                c
            }
            ("sum", []) => {
                let Some(Ty::Int(t)) = place_ty(&g.types, &dest) else {
                    continue;
                };
                Consumer::Sum(int(0, t))
            }
            ("count", []) => Consumer::Count(usize_const(0)),
            ("reduce", [cl]) => {
                let Some(max) = extreme(cl) else { continue };
                Consumer::Extreme(max)
            }
            _ => continue,
        };
        let Some(first) = g.consume(&it, consumer, &dest, ret) else {
            continue;
        };
        g.edits.push((At::Call(bi), at_call));
        if g.failed {
            continue;
        }
        let Gen {
            edits, fresh, held: h, ..
        } = g;
        held.extend(h);
        apply(f, edits, fresh);
        f.body.blocks[bi].terminator = Terminator::Goto(first);
    }
    if !held.is_empty() {
        unread_chains(f, &held);
    }
}

/// A closure of core's that neither this pass nor `enum_methods` took, which nothing after knows: refused.
pub fn refuse_core_closures(f: &mut Function) {
    for b in f.body.blocks.iter_mut() {
        for st in b.statements.iter_mut() {
            if matches!(st, Statement::Assign(_, Rvalue::Closure(..))) {
                *st = Statement::Unsupported(ir::Unsupported::of(&st.to_string()));
            }
        }
    }
}

/// The chains' own lines, now read by nothing, gone: each struct, closure and call that built one, and (as the
/// last read of each goes) what it was built from.
fn unread_chains(f: &mut Function, held: &[Local]) {
    loop {
        let unread: Vec<Local> = held
            .iter()
            .copied()
            .filter(|l| ir::occurrences(&f.body.blocks, *l) == 1)
            .collect();
        let mut gone = false;
        for b in f.body.blocks.iter_mut() {
            for st in b.statements.iter_mut() {
                if st.def().is_some_and(|d| unread.contains(&d)) {
                    *st = Statement::Nop;
                    gone = true;
                }
            }
            if let Terminator::Call {
                dest, target: Some(t), ..
            } = &b.terminator
            {
                if dest.as_local().is_some_and(|d| unread.contains(&d)) {
                    b.terminator = Terminator::Goto(*t);
                    gone = true;
                }
            }
        }
        if !gone {
            break;
        }
    }
    for b in f.body.blocks.iter_mut() {
        b.statements.retain(|s| *s != Statement::Nop);
    }
    let blocks = &f.body.blocks;
    f.body
        .decls
        .retain(|d| !held.contains(&d.local) || ir::occurrences(blocks, d.local) > 0);
}

/// What a chain holds, copied before each line that built it, and the loop's code added.
fn apply(f: &mut Function, edits: Vec<(At, Vec<Statement>)>, mut fresh: Fresh) {
    let mut lines: HashMap<(usize, usize), Vec<Statement>> = HashMap::new();
    for (at, snaps) in edits {
        match at {
            At::Line(bi, si) => lines.entry((bi, si)).or_default().extend(snaps),
            At::Call(bi) => f.body.blocks[bi].statements.extend(snaps),
        }
    }
    for (bi, b) in f.body.blocks.iter_mut().enumerate() {
        if !lines.keys().any(|(x, _)| *x == bi) {
            continue;
        }
        let old = std::mem::take(&mut b.statements);
        for (si, s) in old.into_iter().enumerate() {
            if let Some(snaps) = lines.remove(&(bi, si)) {
                b.statements.extend(snaps);
            }
            b.statements.push(s);
        }
    }
    f.body.decls.append(&mut fresh.decls);
    f.body.blocks.append(&mut fresh.blocks);
}
