//! The front end: from a crate's MIR, parsed (`ir::File`), to the functions the passes work on. On the way, the
//! file is put in the terms the passes expect:
//!   a switch arm, which MIR prints as its bit pattern (65535 for -1 of an i16), is the value in the switch's type
//!   i32, Rust's default integer, is i16: on CARDIAC both are one cell, -999..999
//!   std's paths to what it takes from core (`std::ops::Range`) are core's
//!   `Offset(p, n)` on a raw pointer is `Add(p, n)`: a pointer is a cell address and every element is one cell
//!   a use of a struct const or an array const is the value itself
//!   a function in a module (`kit::arith::twice`) is named by its path joined with `__`, at its definition and at
//!   every call; so is a trait's function in an impl, as the driver names it (`<P as Twice>::twice` is
//!   `P_as_Twice__twice`)
//!   a check of an unsafe precondition, as core makes with debug assertions on, is an assert of what it checks
//!   (`preconditions`)
//! The named scalar consts are read here too, for the lowering to look up.
use std::collections::HashMap;

use super::ir::visit_mut::{walk_const, walk_operand, walk_path, walk_rvalue, walk_terminator, walk_ty, VisitMut};
use super::ir::{
    ArrayLen, BinOp, Block, Body, Const, ConstItem, ConstValue, File, IntTy, Item, Local, Operand, Path, Place,
    Projection, Rvalue, Segment, Statement, Terminator, Ty,
};
use super::structs::Function;
use super::util::Ordered;
use super::Fail;

/// The modules of std that are core's under another name.
const CORE_MODULES: [&str; 14] = [
    "ops",
    "iter",
    "intrinsics",
    "hint",
    "ptr",
    "mem",
    "option",
    "cmp",
    "num",
    "slice",
    "array",
    "panic",
    "marker",
    "clone",
];

/// What the front end makes of a file: the file, its functions by name, and the named scalar consts' values.
pub type FrontEnd = (File, Ordered<String, Function>, HashMap<String, i64>);

/// The file's functions, by name, in the order the file gives them, and the named scalar consts' values.
pub fn front_end(mut file: File, debug_assertions: bool, enums: &[super::enums::EnumDef]) -> Result<FrontEnd, Fail> {
    promoted_consts(&mut file);
    ne_is_not_eq(&mut file);
    super::preconditions::precondition_checks(&mut file, debug_assertions);
    // switch arms are read as their switch's type first: a discriminant's is isize
    switch_values(&mut file);
    cold_paths(&mut file);
    super::enums::c_like_enums(&mut file, enums)?;
    file.visit(&mut TupleStructConsts(enums, &super::enums::normalized(enums)));
    file.visit(&mut I32IsI16);
    file.visit(&mut PastACardDivisor);
    file.visit(&mut StdIsCore);
    file.visit(&mut OffsetIsAdd);
    file.visit(&mut NoUnwinding);
    let named = scalar_consts(&file)?;
    inline_struct_consts(&mut file);
    inline_array_consts(&mut file);
    join_module_paths(&mut file);
    let fns = functions(&file);
    Ok((file, fns, named))
}

/// `<T as PartialEq>::ne(a, b)`, which rustc calls at opt-level 1 and s for a derived `!=`: `PartialEq`'s own `ne`,
/// `!self.eq(other)`, whose body is core's, so it is that: the call to `T`'s `eq`, then its result negated.
fn ne_is_not_eq(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let mut next_local = f
            .params
            .iter()
            .map(|(l, _)| l.0)
            .chain(f.body.decls.iter().map(|d| d.local.0))
            .max()
            .unwrap_or(0)
            + 1;
        let mut next_block = f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
        let mut extra = vec![];
        for b in f.body.blocks.iter_mut() {
            let Terminator::Call {
                dest,
                func,
                target: Some(target),
                ..
            } = &mut b.terminator
            else {
                continue;
            };
            let partial_eq = func.qself.as_ref().is_some_and(|q| {
                q.as_trait
                    .as_ref()
                    .is_some_and(|t| t.name() == "PartialEq" && t.segments.len() == 1)
            });
            let ne = matches!(func.segments.as_slice(), [Segment::Name { name, args: None }] if name == "ne");
            if !(partial_eq && ne) {
                continue;
            }
            let Some(Segment::Name { name, .. }) = func.segments.last_mut() else {
                continue;
            };
            *name = "eq".into();
            let equal = Local(next_local);
            next_local += 1;
            f.body.decls.push(super::ir::Decl {
                local: equal,
                mutable: false,
                ty: Ty::Bool,
            });
            let then = super::ir::BasicBlock(next_block);
            next_block += 1;
            extra.push(super::ir::Block {
                name: then,
                statements: vec![Statement::Assign(
                    dest.clone(),
                    Rvalue::UnaryOp(super::ir::UnOp::Not, Operand::Move(Place::local(equal))),
                )],
                terminator: Terminator::Goto(*target),
            });
            *dest = Place::local(equal);
            *target = then;
        }
        f.body.blocks.extend(extra);
    }
}

/// Each promoted constant a function uses, `const main::promoted[1]`: a reference to a value rustc built once, in
/// a constant of its own (a struct literal passed by reference, `show(&P { .. })`; `&Colour::Green` passed to a
/// derived `eq`). The value is built in the function instead, in cells of its own at the start of the block
/// that uses it, and borrowed there: the promoted's body (`_1 = P { a: const 3_i16, .. }; _0 = &_1;`) with its
/// locals renumbered as the function's, `_0` the borrow. A promoted of another shape (more than one block) stays.
fn promoted_consts(file: &mut File) {
    // each promoted's type and body, by its name: kept as rustc's text, `const main::promoted[0]: &P = {` and its body
    let mut promoted: HashMap<String, (Ty, Body)> = HashMap::new();
    for item in &file.items {
        let Item::Other(text) = item else { continue };
        let Some((head, rest)) = text.split_once('\n') else {
            continue;
        };
        let Some(head) = head.strip_prefix("const ").and_then(|h| h.strip_suffix(" = {")) else {
            continue;
        };
        let Some((name, ty)) = head.split_once(": ") else {
            continue;
        };
        if !name.contains("::promoted[") {
            continue;
        }
        let (Ok(ty), Some(body)) = (super::ir::parse_ty(ty), rest.strip_suffix("\n}")) else {
            continue;
        };
        let mut body = super::ir::parse_body(body);
        range_new_as_literal(&mut body);
        promoted.insert(name.to_string(), (ty, body));
    }
    if promoted.is_empty() {
        return;
    }
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
            super::ir::visit_mut::walk_place(self, p)
        }
    }
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let mut next = f
            .params
            .iter()
            .map(|(l, _)| l.0)
            .chain(f.body.decls.iter().map(|d| d.local.0))
            .max()
            .unwrap_or(0)
            + 1;
        let mut decls = vec![];
        for b in f.body.blocks.iter_mut() {
            let mut setup = vec![];
            let mut operands: Vec<&mut Operand> = b.statements.iter_mut().flat_map(|s| s.operands_mut()).collect();
            operands.extend(b.terminator.operands_mut());
            for o in operands {
                let Operand::Constant(Const::Promoted(function, n)) = &*o else {
                    continue;
                };
                let Some((ty, body)) = promoted.get(&format!("{function}::promoted[{n}]")) else {
                    continue;
                };
                let [only] = body.blocks.as_slice() else { continue };
                if only.terminator != Terminator::Return {
                    continue;
                }
                // its locals as the function's: _0, the borrow, too
                let mut to: HashMap<Local, Local> = HashMap::new();
                to.insert(Local(0), Local(next));
                decls.push(super::ir::Decl {
                    local: Local(next),
                    mutable: false,
                    ty: ty.clone(),
                });
                next += 1;
                for d in &body.decls {
                    if d.local != Local(0) {
                        to.insert(d.local, Local(next));
                        decls.push(super::ir::Decl {
                            local: Local(next),
                            mutable: true,
                            ty: d.ty.clone(),
                        });
                        next += 1;
                    }
                }
                let mut statements = only.statements.clone();
                for s in statements.iter_mut() {
                    Renumber(&to).visit_statement(s);
                }
                setup.extend(statements);
                *o = Operand::Move(Place::local(to[&Local(0)]));
            }
            if !setup.is_empty() {
                setup.append(&mut b.statements);
                b.statements = setup;
            }
        }
        f.body.decls.extend(decls);
    }
}

/// A promoted `&(1..=13)`, as rustc builds it: `_1 = core::ops::RangeInclusive::<i16>::new(const 1_i16, const
/// 13_i16) -> bb1`, then the borrow. The call is the range's literal, `RangeInclusive::<i16> {{ start, end,
/// exhausted: false }}` (as `for` loops read one, forloops.rs), so the promoted is one block, which is inlined.
fn range_new_as_literal(body: &mut Body) {
    let [first, second] = body.blocks.as_slice() else {
        return;
    };
    let Terminator::Call {
        dest,
        func,
        args,
        target: Some(next),
        ..
    } = &first.terminator
    else {
        return;
    };
    let (Some((new, ty)), [a, b]) = (func.segments.split_last(), args.as_slice()) else {
        return;
    };
    let range = matches!(ty, [Segment::Name { name: c, args: None }, Segment::Name { name: o, args: None }, Segment::Name { name: r, args: Some(_) }]
        if (c == "core" || c == "std") && o == "ops" && r == "RangeInclusive");
    if func.qself.is_some()
        || !range
        || !matches!(new, Segment::Name { name, args: None } if name == "new")
        || *next != second.name
    {
        return;
    }
    let path = Path {
        qself: None,
        segments: ty.to_vec(),
    };
    let fields = vec![
        ("start".to_string(), a.clone()),
        ("end".to_string(), b.clone()),
        ("exhausted".to_string(), Operand::Constant(Const::Bool(false))),
    ];
    let mut statements = first.statements.clone();
    statements.push(Statement::Assign(dest.clone(), Rvalue::Struct(path, fields)));
    statements.extend(second.statements.clone());
    let one = Block {
        name: first.name,
        statements,
        terminator: second.terminator.clone(),
    };
    body.blocks = vec![one];
}

/// Each switch arm as a value of the switch's type: MIR prints an arm as its bit pattern, `switchInt(copy _2) ->
/// [65535: bb3, ..]` for `x == -1` with x an i16 (4294967295 for an i32). Done first, while an i32 is still one: its
/// pattern is 32 bits.
fn switch_values(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let types: HashMap<Local, Ty> = f
            .params
            .iter()
            .cloned()
            .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
            .collect();
        for b in &mut f.body.blocks {
            let Terminator::SwitchInt { discr, arms, .. } = &mut b.terminator else {
                continue;
            };
            let ty = match discr {
                Operand::Constant(Const::Int { ty, .. }) => Some(*ty),
                _ => discr.place().and_then(|p| place_ty(&types, p)).and_then(|t| match t {
                    Ty::Int(t) => Some(t),
                    _ => None,
                }),
            };
            if let Some(ty) = ty {
                for (v, _) in arms.iter_mut() {
                    *v = ty.from_bits(*v);
                }
            }
        }
    }
}

/// `core::intrinsics::cold_path()`, which marks a way as rarely taken (`unwrap()`'s panic, at opt-level 2): no code,
/// a jump on.
fn cold_paths(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        for b in &mut f.body.blocks {
            if let Terminator::Call {
                func, target: Some(t), ..
            } = &b.terminator
            {
                if func.is_path(&["core", "intrinsics", "cold_path"]) || func.is_path(&["cold_path"]) {
                    b.terminator = Terminator::Goto(*t);
                }
            }
        }
    }
}

/// The type of a place, from its local's through its projections.
fn place_ty(types: &HashMap<Local, Ty>, p: &Place) -> Option<Ty> {
    let mut t = types.get(&p.base_local()?)?.clone();
    for x in &p.projection {
        t = match (x, t) {
            (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
            (Projection::Field(_, f), _) => f.clone(),
            (Projection::Downcast(_), t) => t,
            (
                Projection::Index(_)
                | Projection::IndexPlus(..)
                | Projection::ConstantIndex { .. }
                | Projection::IndexPlace(_),
                Ty::Array(e, _) | Ty::Slice(e),
            ) => *e,
            _ => return None,
        };
    }
    Some(t)
}

/// A call or a check in a function that can't unwind (`extern "C"`), whose panic Rust turns into an abort
/// (`unwind terminate(abi)`): CARDIAC never unwinds, and a panic stops the machine where it is, as anywhere else.
/// (Natively the abort exits 134, not a panic's 101; the output is the same.)
struct NoUnwinding;

impl VisitMut for NoUnwinding {
    fn visit_terminator(&mut self, t: &mut Terminator) {
        if let Terminator::Call { unwind, .. } | Terminator::Assert { unwind, .. } = t {
            if matches!(unwind, super::ir::Unwind::Terminate(why) if why == "abi") {
                *unwind = super::ir::Unwind::Continue;
            }
        }
        walk_terminator(self, t)
    }
}

/// `x % k` and `x / k` of an i16 with k past a card (`% 1000`): a card's size is at most 999, so `x % k` is x (Rust's
/// `%` keeps the dividend's sign) and `x / k` is 0, and k never needs a cell, which it couldn't have.
struct PastACardDivisor;

impl VisitMut for PastACardDivisor {
    fn visit_rvalue(&mut self, r: &mut Rvalue) {
        walk_rvalue(self, r);
        if let Rvalue::BinaryOp(
            op @ (BinOp::Rem | BinOp::Div),
            a,
            Operand::Constant(Const::Int { value, ty: IntTy::I16 }),
        ) = r
        {
            if value.abs() > 999 {
                *r = if *op == BinOp::Rem {
                    Rvalue::Use(a.clone())
                } else {
                    Rvalue::Use(Operand::Constant(Const::Int {
                        value: 0,
                        ty: IntTy::I16,
                    }))
                };
            }
        }
    }
}

/// A tuple struct's constant, `const Flag(-1_i16)`, and its building, `Flag(move _1)`, read as a variant's
/// (`Err(-9_i16)`): where the path names no enum's variant they are the struct's, its fields `0`, `1`, ... as a
/// tuple struct's are named. An enum is looked for as the MIR names it and as the C-like enums pass has made it
/// (`Option::<i16>` for `Option::<Colour>`): the definitions as given, and normalized.
struct TupleStructConsts<'a>(&'a [super::enums::EnumDef], &'a [super::enums::EnumDef]);

impl TupleStructConsts<'_> {
    fn names_a_variant(&self, p: &Path) -> bool {
        super::enums::names_a_variant(self.0, p) || super::enums::names_a_variant(self.1, p)
    }
}

impl VisitMut for TupleStructConsts<'_> {
    fn visit_rvalue(&mut self, r: &mut Rvalue) {
        if let Rvalue::Variant(p, ops) = r {
            if !ops.is_empty() && !self.names_a_variant(p) {
                let fields = ops.drain(..).enumerate().map(|(i, o)| (i.to_string(), o)).collect();
                *r = Rvalue::Struct(p.clone(), fields);
            }
        }
        walk_rvalue(self, r)
    }

    fn visit_const(&mut self, c: &mut Const) {
        if let Const::Variant(p, cs) = c {
            if !cs.is_empty() && !self.names_a_variant(p) {
                let fields = cs.drain(..).enumerate().map(|(i, c)| (i.to_string(), c)).collect();
                *c = Const::Adt(p.clone(), fields);
            }
        }
        walk_const(self, c)
    }
}

struct I32IsI16;

impl VisitMut for I32IsI16 {
    fn visit_ty(&mut self, t: &mut Ty) {
        if *t == Ty::Int(IntTy::I32) {
            *t = Ty::Int(IntTy::I16);
        }
        walk_ty(self, t)
    }

    fn visit_const(&mut self, c: &mut Const) {
        if let Const::Int {
            ty: ty @ IntTy::I32, ..
        } = c
        {
            *ty = IntTy::I16;
        }
        walk_const(self, c)
    }

    /// `i32::MAX`
    fn visit_path(&mut self, p: &mut Path) {
        for s in &mut p.segments {
            if let Segment::Name { name, .. } = s {
                if name == "i32" {
                    *name = "i16".into();
                }
            }
        }
        walk_path(self, p)
    }
}

struct StdIsCore;

impl VisitMut for StdIsCore {
    fn visit_path(&mut self, p: &mut Path) {
        if p.qself.is_none() && p.segments.len() > 2 {
            if let [Segment::Name {
                name: krate,
                args: None,
            }, Segment::Name { name: module, .. }, ..] = p.segments.as_mut_slice()
            {
                if krate == "std" && CORE_MODULES.contains(&module.as_str()) {
                    *krate = "core".into();
                }
            }
        }
        walk_path(self, p)
    }
}

struct OffsetIsAdd;

impl VisitMut for OffsetIsAdd {
    fn visit_rvalue(&mut self, r: &mut Rvalue) {
        if let Rvalue::BinaryOp(op @ BinOp::Offset, ..) = r {
            *op = BinOp::Add;
        }
        walk_rvalue(self, r)
    }
}

/// `const LIMIT: i16 = const 40_i16;`: a const of one name and a scalar value. Its value must be a u8, i16, usize or
/// bool.
fn scalar_consts(file: &File) -> Result<HashMap<String, i64>, Fail> {
    let mut named = HashMap::new();
    for item in &file.items {
        let Item::Const(ConstItem { path, ty, value }) = item else {
            continue;
        };
        let Some(name) = path.single().filter(|_| path.is_plain()) else {
            continue;
        };
        let v = match value {
            ConstValue::Const(Const::Int {
                value,
                ty: IntTy::U8 | IntTy::I16 | IntTy::Usize,
            }) => *value as i64,
            ConstValue::Const(Const::Bool(b)) => *b as i64,
            ConstValue::Const(Const::Int { .. }) => return Err(format!("unsupported const {name}: {ty}")),
            _ if matches!(ty, Ty::Path(p) if p.is("f32") || p.is("f64")) => {
                return Err(format!("unsupported const {name}: {ty}"))
            }
            _ => continue,
        };
        named.insert(name.to_string(), v);
    }
    Ok(named)
}

/// A const's name as a use of it gives it, `const ORIGIN` or `const kit::Point::ORIGIN`: a path of plain names,
/// the last in capitals.
fn const_path(p: &Path) -> Option<Vec<&str>> {
    if p.qself.is_some() {
        return None;
    }
    let names: Vec<&str> = p
        .segments
        .iter()
        .map(|s| match s {
            Segment::Name { name, args: None } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let last = names.last()?.as_bytes();
    let capitals = (last[0].is_ascii_uppercase() || last[0] == b'_')
        && last
            .iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_');
    capitals.then_some(names)
}

/// The first statement of a body that assigns its result, `_0 = ...`.
fn result_value(body: &Body) -> impl Iterator<Item = &Rvalue> {
    body.blocks.iter().flat_map(|b| &b.statements).filter_map(|s| match s {
        Statement::Assign(p, r) if p.as_local() == Some(Local(0)) => Some(r),
        _ => None,
    })
}

/// A path of one plain name.
fn one_name(p: &Path) -> bool {
    p.qself.is_none() && matches!(p.segments.as_slice(), [Segment::Name { args: None, .. }])
}

/// Struct consts (`const CORNER: Point = ...`, `impl Point { const ORIGIN: Point = ... }`): each use becomes the
/// struct literal itself, `Point { x: const 0_i16, .. }`, which the struct pass then takes apart like any other. A
/// const in an impl is found by its type's name and its own; any other by its own name.
fn inline_struct_consts(file: &mut File) {
    let mut owned: HashMap<(String, String), Rvalue> = HashMap::new();
    let mut free: HashMap<String, Rvalue> = HashMap::new();
    for item in &file.items {
        let Item::Const(ConstItem {
            path,
            ty,
            value: ConstValue::Body(body),
        }) = item
        else {
            continue;
        };
        // `module::NAME` or `module::<impl at ..>::NAME`
        let (name, before) = path.segments.split_last().unwrap();
        let (impl_at, modules) = match before.split_last() {
            Some((s, modules)) if s.starts_with("<impl at ") => (true, modules),
            _ => (false, before),
        };
        if !plain(name) || !modules.iter().all(|m| plain(m)) {
            continue;
        }
        let ty_name = match ty {
            Ty::Path(p)
                if p.qself.is_none() && p.segments.iter().all(|s| matches!(s, Segment::Name { args: None, .. })) =>
            {
                p.name().to_string()
            }
            Ty::Int(_) | Ty::Bool => ty.to_string(),
            _ => continue,
        };
        let literal = result_value(body).find(|r| match r {
            Rvalue::Struct(p, fields) => one_name(p) && fields.iter().all(|(_, o)| !braces(o)),
            _ => false,
        });
        let Some(literal) = literal else { continue };
        if impl_at {
            owned.insert((ty_name, name.clone()), literal.clone());
        } else {
            free.insert(name.clone(), literal.clone());
        }
    }
    if owned.is_empty() && free.is_empty() {
        return;
    }
    struct Inline<'a> {
        owned: &'a HashMap<(String, String), Rvalue>,
        free: &'a HashMap<String, Rvalue>,
    }
    impl Inline<'_> {
        fn literal(&self, o: &Operand) -> Option<&Rvalue> {
            let Operand::Constant(Const::Path(p)) = o else {
                return None;
            };
            let names = const_path(p)?;
            let (name, before) = names.split_last().unwrap();
            // with no owner, an owned const is still looked for, under the owner "undefined"
            let owner = before.last().copied().unwrap_or("undefined");
            self.owned
                .get(&(owner.to_string(), name.to_string()))
                .or_else(|| self.free.get(*name))
        }
    }
    impl VisitMut for Inline<'_> {
        fn visit_rvalue(&mut self, r: &mut Rvalue) {
            if let Rvalue::Use(o) = r {
                if let Some(literal) = self.literal(o) {
                    *r = literal.clone();
                    return;
                }
            }
            walk_rvalue(self, r)
        }

        fn visit_operand(&mut self, o: &mut Operand) {
            if let Some(Rvalue::Struct(p, fields)) = self.literal(o) {
                *o = Operand::Aggregate(p.clone(), fields.clone());
                return;
            }
            walk_operand(self, o)
        }
    }
    file.visit(&mut Inline {
        owned: &owned,
        free: &free,
    });
}

/// A name of letters, digits and underscores.
fn plain(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

/// An operand written with braces: a struct value inside a struct value.
fn braces(o: &Operand) -> bool {
    matches!(o, Operand::Aggregate(..) | Operand::Constant(Const::Adt(..)))
}

/// Named array consts (a table in a library module, say): `const T: [i16; N] = { ... _0 = [...]; }`. Each use,
/// `const T` or `const path::T`, becomes the array itself, which then works as any array literal does (in main's
/// opening blocks, the cells' initial values). A scalar const used by path loses the path.
fn inline_array_consts(file: &mut File) {
    let mut arrays: HashMap<String, Rvalue> = HashMap::new();
    for item in &file.items {
        let Item::Const(ConstItem {
            path,
            ty: Ty::Array(elem, ArrayLen::Known(_)),
            value: ConstValue::Body(body),
        }) = item
        else {
            continue;
        };
        let Some(name) = path.single().filter(|_| path.is_plain()) else {
            continue;
        };
        if !matches!(**elem, Ty::Int(IntTy::I16 | IntTy::U8)) {
            continue;
        }
        let constant = |o: &Operand| matches!(o, Operand::Constant(c) if !matches!(c, Const::Adt(..)));
        let literal = result_value(body).find(|r| match r {
            Rvalue::Array(os) => os.iter().all(constant),
            Rvalue::Repeat(o, _) => constant(o),
            _ => false,
        });
        if let Some(literal) = literal {
            arrays.insert(name.to_string(), literal.clone());
        }
    }
    struct Inline<'a> {
        arrays: &'a HashMap<String, Rvalue>,
    }
    /// A path to a const in another module, `tables::SQUARES` (not `i16::MAX`).
    fn qualified(names: &[&str]) -> bool {
        names.len() > 1 && !["i16", "u8", "usize", "i32", "u16", "isize"].contains(&names[0])
    }
    impl VisitMut for Inline<'_> {
        fn visit_rvalue(&mut self, r: &mut Rvalue) {
            if let Rvalue::Use(Operand::Constant(Const::Path(p))) = r {
                if let Some(names) = const_path(p) {
                    if let Some(array) = self
                        .arrays
                        .get(*names.last().unwrap())
                        .filter(|_| names.len() == 1 || qualified(&names))
                    {
                        *r = array.clone();
                        return;
                    }
                }
            }
            walk_rvalue(self, r)
        }

        fn visit_operand(&mut self, o: &mut Operand) {
            if let Operand::Constant(Const::Path(p)) = o {
                if let Some(names) = const_path(p).filter(|n| qualified(n)) {
                    let name = names.last().unwrap().to_string();
                    p.segments = vec![Segment::Name { name, args: None }];
                    return;
                }
            }
            walk_operand(self, o)
        }
    }
    file.visit(&mut Inline { arrays: &arrays });
}

/// `kit::arith::twice` is `kit__arith__twice`, where it is defined and where it is called (core's and std's
/// functions keep their paths). A trait's function in an impl, which the CARDIAC driver names as a call to it does
/// (`<P as Twice>::twice`), is a name of the same kind, `P_as_Twice__twice`, at its definition and at each call
/// to it.
fn join_module_paths(file: &mut File) {
    let mut impls: HashMap<String, String> = HashMap::new();
    for item in &mut file.items {
        if let Item::Fn(f) = item {
            if f.path.segments.len() > 1 && f.path.is_plain() {
                f.path.segments = vec![f.path.segments.join("__")];
            } else if let Some(name) = impl_fn_name(&f.path.to_string()) {
                impls.insert(f.path.to_string(), name.clone());
                f.path.segments = vec![name];
            }
        }
    }
    struct Calls<'a> {
        impls: &'a HashMap<String, String>,
    }
    impl VisitMut for Calls<'_> {
        fn visit_terminator(&mut self, t: &mut Terminator) {
            if let Terminator::Call { func, .. } = t {
                let names: Option<Vec<String>> = func
                    .segments
                    .iter()
                    .map(|s| match s {
                        Segment::Name { name, args: None } => Some(name.clone()),
                        _ => None,
                    })
                    .collect();
                if let Some(name) = func
                    .qself
                    .is_some()
                    .then(|| self.impls.get(&func.to_string()))
                    .flatten()
                {
                    *func = Path {
                        qself: None,
                        segments: vec![Segment::Name {
                            name: name.clone(),
                            args: None,
                        }],
                    };
                } else if let Some(names) =
                    // (core's own, trimmed, as `option::unwrap_failed`, stay as they are)
                    names.filter(|n| {
                        func.qself.is_none()
                            && n.len() > 1
                            && n[0] != "core"
                            && n[0] != "std"
                            && !super::lower::calls::never_returns(func)
                    })
                {
                    func.segments = vec![Segment::Name {
                        name: names.join("__"),
                        args: None,
                    }];
                }
            }
            walk_terminator(self, t)
        }
    }
    file.visit(&mut Calls { impls: &impls });
}

/// `<P as Twice>::twice` as one plain name, `P_as_Twice__twice`: for a type and a trait each named by plain words.
fn impl_fn_name(path: &str) -> Option<String> {
    let rest = path.strip_prefix('<')?;
    let (qself, name) = rest.split_once(">::")?;
    let (ty, tr) = qself.split_once(" as ")?;
    let word = |w: &str| {
        !w.is_empty()
            && w.split("::")
                .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
    };
    (word(ty) && word(tr) && word(name)).then(|| {
        format!(
            "{}_as_{}__{}",
            ty.replace("::", "__"),
            tr.replace("::", "__"),
            name.replace("::", "__")
        )
    })
}

/// A plain name: letters, digits and `_`.
fn word(w: &str) -> bool {
    !w.is_empty() && w.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

/// The functions the compiler reads: those named by one plain name (not an impl's), and closures' (as
/// `OWNER_closureN`), in the order the file gives them. Of two of the same name (a const fn and its `// MIR FOR CTFE` copy), the later body is the one kept, in
/// the earlier one's place.
fn functions(file: &File) -> Ordered<String, Function> {
    let mut fns = Ordered::new();
    for item in &file.items {
        let Item::Fn(f) = item else { continue };
        let name = match f.path.segments.as_slice() {
            // a closure's own function, `main::{closure#0}`, as `main_closure0` (a call to it is `map`'s)
            [owner, c] if word(owner) => match c.strip_prefix("{closure#").and_then(|c| c.strip_suffix('}')) {
                Some(n) if n.bytes().all(|b| b.is_ascii_digit()) => format!("{owner}_closure{n}"),
                _ => continue,
            },
            _ => match f.path.single().filter(|_| f.path.is_plain()) {
                Some(name) => name.to_string(),
                None => continue,
            },
        };
        let name = name.as_str();
        if f.unread() {
            continue;
        }
        fns.set(
            name.to_string(),
            Function {
                name: name.to_string(),
                params: f.params.clone(),
                ret: f.ret.clone(),
                body: f.body.clone(),
                results: vec![],
            },
        );
    }
    fns
}
