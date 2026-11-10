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
//!   a panic with a literal message, and `assert_eq!`'s, is `core::panicking::panic` of the message
//!   (`panic_messages`)
//! The named scalar consts are read here too, for the lowering to look up.
use std::collections::HashMap;

use super::ir::visit_mut::{walk_const, walk_operand, walk_path, walk_rvalue, walk_terminator, walk_ty, VisitMut};
use super::ir::{
    ArrayLen, BinOp, Block, Body, CastKind, Const, ConstItem, ConstValue, File, IntTy, Item, ItemPath, Local, Operand,
    Path, Place, PlaceBase, Projection, Rvalue, Segment, Statement, Terminator, Ty, UnOp,
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

/// What the front end makes of a file: the file, its functions by name, the named scalar consts' values, its
/// statics, the enums its trait objects are (`indirect::trait_objects`), and the function of each closure that
/// captures, by the closure's type (`closures::closures_as_structs`).
pub type FrontEnd = (
    File,
    Ordered<String, Function>,
    HashMap<String, i64>,
    Statics,
    Vec<super::enums::EnumDef>,
    HashMap<String, String>,
);

/// The program's statics, each a fixed run of cells, as every variable on CARDIAC is: what each holds first, by the
/// static's name, and the static each allocation a body points into is (`{alloc1: *mut i16}`).
#[derive(Clone, Debug, Default)]
pub struct Statics {
    pub values: Ordered<String, Vec<i64>>,
    pub of_alloc: HashMap<String, String>,
}

/// The overflow checks that codegen leaves out where the build's overflow checks are off: `+`, `-`, `*`, a
/// negation and the shifts. rustc writes none in a crate built without them, but it keeps those of core's code it
/// inlines that inherits the caller's (`#[rustc_inherit_overflow_checks]`, as `next_power_of_two`'s `+ 1` does),
/// and the build that runs natively doesn't check them. A divide's are always made.
const INHERITED_CHECKS: [&str; 6] = [
    "attempt to compute `{} + {}`, which would overflow",
    "attempt to compute `{} - {}`, which would overflow",
    "attempt to compute `{} * {}`, which would overflow",
    "attempt to negate `{}`, which would overflow",
    "attempt to shift left by `{}`, which would overflow",
    "attempt to shift right by `{}`, which would overflow",
];

/// For a build without overflow checks (the driver knows the session's): each of those checks goes on to its
/// target, as the native build does. The checked operation it read the flag of is then the plain one, into the
/// pair's value, where nothing else reads the flag (a u8's then wraps, `ranges::mark_wrapping`).
pub fn drop_inherited_overflow_checks(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let body = &mut f.body;
        for bi in 0..body.blocks.len() {
            let Terminator::Assert { cond, msg, target, .. } = &body.blocks[bi].terminator else {
                continue;
            };
            if !INHERITED_CHECKS.contains(&msg.as_str()) {
                continue;
            }
            // the flag, `_t.1`
            let flag = cond
                .place()
                .filter(|p| matches!(p.projection.as_slice(), [Projection::Field(1, _)]))
                .and_then(|p| p.base_local());
            body.blocks[bi].terminator = Terminator::Goto(*target);
            let Some(t) = flag else { continue };
            let read_whole_or_flag = body.blocks.iter().any(|b| {
                // what each statement reads: its places but the one an assignment writes
                let read = b.statements.iter().flat_map(|s| {
                    let skip = usize::from(matches!(s, Statement::Assign(..)));
                    s.places().into_iter().skip(skip)
                });
                read.chain(b.terminator.places()).any(|p| {
                    p.base_local() == Some(t) && !matches!(p.projection.first(), Some(Projection::Field(0, _)))
                })
            });
            let Some(Ty::Tuple(fields)) = body.decls.iter().find(|d| d.local == t).map(|d| &d.ty) else {
                continue;
            };
            let (false, Some(value)) = (read_whole_or_flag, fields.first().cloned()) else {
                continue;
            };
            for s in &mut body.blocks[bi].statements {
                if let Statement::Assign(d, Rvalue::BinaryOp(op, a, c)) = s {
                    let plain = match op {
                        BinOp::AddWithOverflow => BinOp::Add,
                        BinOp::SubWithOverflow => BinOp::Sub,
                        BinOp::MulWithOverflow => BinOp::Mul,
                        _ => continue,
                    };
                    if d.as_local() == Some(t) {
                        *s = Statement::Assign(
                            Place::field(t, 0, value.clone()),
                            Rvalue::BinaryOp(plain, a.clone(), c.clone()),
                        );
                    }
                }
            }
        }
    }
}

/// The file's functions, by name, in the order the file gives them, and the named scalar consts' values.
pub fn front_end(
    mut file: File,
    debug_assertions: bool,
    enums: &[super::enums::EnumDef],
    structs: &[super::structs::StructDef],
) -> Result<FrontEnd, Fail> {
    // the names of the program's own types, which a type of core's trimmed to its name (`Reverse<i16>`) isn't
    let own: Vec<String> = structs
        .iter()
        .filter(|s| s.in_crate)
        .map(|s| s.name.clone())
        .chain(
            enums
                .iter()
                .filter(|e| e.in_crate)
                .filter_map(|e| e.path.last().cloned()),
        )
        .collect();
    promoted_consts(&mut file);
    // function pointers as numbers, and trait objects as enums, with calls through them as calls of what they can
    // be (after the promoteds, which may make them)
    super::indirect::function_pointers(&mut file);
    let mut objects = super::indirect::trait_objects(&mut file, structs);
    // a closure that captures, a struct; one called through its trait, a call of its function
    let capturing = super::closures::closures_as_structs(&mut file);
    // a trait object's closure holds its struct (one that captures nothing is left, for the enum pass to refuse)
    for t in objects
        .iter_mut()
        .flat_map(|o| &mut o.variants)
        .flat_map(|v| &mut v.fields)
    {
        if let Some(function) = match t {
            Ty::Other(closure) => capturing.get(closure),
            _ => None,
        } {
            *t = super::closures::env_struct(function);
        }
    }
    super::panic_messages::literal_panics(&mut file);
    strings_as_bytes(&mut file);
    byte_strings(&mut file);
    file.visit(&mut CharIsUsize);
    // a char's to_digit and from_digit, written out (with the `Option<u8>` the first returns)
    objects.extend(super::chars::char_digits(&mut file, enums));
    super::nonzero::nonzero_is_its_value(&mut file, &own);
    ne_is_not_eq(&mut file);
    super::preconditions::precondition_checks(&mut file, debug_assertions);
    // switch arms are read as their switch's type first: a discriminant's is isize
    switch_values(&mut file);
    cold_paths(&mut file);
    // `Cmp` and `Ordering`'s methods, written with its variants, which the C-like pass numbers
    super::ordering::reverse_is_its_value(&mut file, &own);
    super::ordering::three_way_compares(&mut file);
    // a cycle of functions that call one another at their ends, one function; then a function's call of itself
    // there, a loop
    super::cycles::mutual_tail_calls(&mut file);
    tail_calls(&mut file);
    super::enums::c_like_enums(&mut file, enums)?;
    file.visit(&mut TupleStructConsts(enums, &super::enums::normalized(enums)));
    let widened = widened_in_core(&file);
    // u16 and u32 locals as cells, before an i32 is an i16: a cast to i32 is a cast to a wider type; first those
    // `wide_locals` can't make cells, as all a u16's values are
    super::wide::u16_cells(&mut file);
    super::wide::wide_locals(&mut file);
    file.visit(&mut I32IsI16);
    file.visit(&mut IsizeIsI16);
    file.visit(&mut PastACardDivisor);
    file.visit(&mut PastACardBound);
    file.visit(&mut StdIsCore);
    file.visit(&mut OffsetIsAdd);
    file.visit(&mut NoUnwinding);
    let named = scalar_consts(&file)?;
    let statics = statics(&file);
    inline_struct_consts(&mut file);
    inline_array_consts(&mut file);
    join_module_paths(&mut file);
    let mut fns = functions(&file);
    for (name, p) in widened {
        if let Some(f) = fns.get_mut(&name) {
            f.widened = Some(p);
        }
    }
    Ok((file, fns, named, statics, objects, capturing))
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

/// A `&str` local is the slice of its bytes, `&[u8]`, and a string literal it is set to (or `as_bytes()` of one,
/// a transmute) the byte string of the literal's bytes. A literal with an escape other than
/// `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\'` and an ASCII `\x`, or a character past ASCII, is left as it is.
fn strings_as_bytes(file: &mut File) {
    fn bytes(s: &str) -> Option<Vec<u8>> {
        let mut out = vec![];
        let mut cs = s.chars();
        while let Some(c) = cs.next() {
            out.push(match c {
                '\\' => match cs.next()? {
                    'n' => b'\n',
                    't' => b'\t',
                    'r' => b'\r',
                    '0' => 0,
                    c @ ('\\' | '"' | '\'') => c as u8,
                    'x' => {
                        let h: String = [cs.next()?, cs.next()?].iter().collect();
                        u8::from_str_radix(&h, 16).ok().filter(|b| b.is_ascii())?
                    }
                    _ => return None,
                },
                c if c.is_ascii() => c as u8,
                _ => return None,
            });
        }
        Some(out)
    }
    let str_ref = |t: &Ty| matches!(t, Ty::Ref { to, .. } if **to == Ty::Str);
    let bytes_ref = |t: &Ty| matches!(t, Ty::Ref { to, .. } if **to == Ty::Slice(Box::new(Ty::Int(IntTy::U8))));
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
        types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                let Statement::Assign(d, r) = s else { continue };
                let Some(t) = d.as_local().and_then(|l| types.get(&l)) else { continue };
                let literal = match r {
                    Rvalue::Use(Operand::Constant(Const::Str(s))) if str_ref(t) => Some(s),
                    Rvalue::Cast(Operand::Constant(Const::Str(s)), _, CastKind::Transmute) if bytes_ref(t) => Some(s),
                    _ => None,
                };
                if let Some(b) = literal.and_then(|s| bytes(s)) {
                    *r = Rvalue::Use(Operand::Constant(Const::Bytes(b)));
                }
            }
        }
        let slice = |t: &mut Ty| {
            if let Ty::Ref { to, .. } = t {
                if **to == Ty::Str {
                    **to = Ty::Slice(Box::new(Ty::Int(IntTy::U8)));
                }
            }
        };
        f.params.iter_mut().for_each(|(_, t)| slice(t));
        f.body.decls.iter_mut().for_each(|d| slice(&mut d.ty));
        slice(&mut f.ret);
    }
}

/// Each byte string a function uses, `const b"CARD"`: a reference to an array of the bytes, which is built in the
/// function, at the start of the block that uses it, in a local of its own (a `[u8; N]` of constants, as a
/// promoted's value is), and borrowed there.
fn byte_strings(file: &mut File) {
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
                // (`b""` too has no array literal: `[]` isn't MIR's)
                let Operand::Constant(Const::Bytes(bytes)) = &*o else {
                    continue;
                };
                if bytes.is_empty() {
                    continue;
                }
                let array_ty = Ty::Array(Box::new(Ty::Int(IntTy::U8)), ArrayLen::Known(bytes.len() as u64));
                let (array, borrow) = (Local(next), Local(next + 1));
                next += 2;
                decls.push(super::ir::Decl {
                    local: array,
                    mutable: false,
                    ty: array_ty.clone(),
                });
                decls.push(super::ir::Decl {
                    local: borrow,
                    mutable: false,
                    ty: Ty::Ref {
                        lifetime: None,
                        mutable: false,
                        to: Box::new(array_ty),
                    },
                });
                let byte = |v: &u8| {
                    Operand::Constant(Const::Int {
                        value: *v as i128,
                        ty: IntTy::U8,
                    })
                };
                setup.push(Statement::Assign(
                    Place::local(array),
                    Rvalue::Array(bytes.iter().map(byte).collect()),
                ));
                setup.push(Statement::Assign(
                    Place::local(borrow),
                    Rvalue::Ref(super::ir::BorrowKind::Shared, Place::local(array)),
                ));
                *o = Operand::Move(Place::local(borrow));
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

/// The types of a function's parameters and locals.
pub(super) fn local_types(params: &[(Local, Ty)], decls: &[super::ir::Decl]) -> HashMap<Local, Ty> {
    params
        .iter()
        .cloned()
        .chain(decls.iter().map(|d| (d.local, d.ty.clone())))
        .collect()
}

/// A function's call of itself whose result is its own, with nothing after it but the way to its return
/// (`count(n - 1, acc + 2)` as the last thing `count` does): a loop. Each argument goes into a temporary, then each
/// temporary into its parameter, and the call is a jump back to the start (a block of its own that the entry goes
/// to, so the entry is entered once, as a loop's header isn't); the function's cells hold the next round's values,
/// as they would the callee's. Only where no parameter or argument is a reference or a pointer,
/// which could point into the cells being reused.
fn tail_calls(file: &mut File) {
    let pointer = |t: &Ty| matches!(t, Ty::Ref { .. } | Ty::Ptr { .. });
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        if !f.path.is_plain() || f.params.iter().any(|(_, t)| pointer(t)) {
            continue;
        }
        if !f.body.blocks.iter().any(|b| b.name == super::ir::BasicBlock(0)) {
            continue;
        }
        let name = f.path.to_string();
        let types: HashMap<Local, Ty> = f
            .params
            .iter()
            .cloned()
            .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
            .collect();
        let mut next = types.keys().map(|l| l.0).max().unwrap_or(0) + 1;
        let mut decls = vec![];
        let blocks = f.body.blocks.clone();
        let start = super::ir::BasicBlock(blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1);
        let mut looped = false;
        for b in f.body.blocks.iter_mut() {
            let Terminator::Call {
                dest,
                func,
                args,
                target: Some(target),
                unwind: super::ir::Unwind::Continue | super::ir::Unwind::Unreachable,
            } = &b.terminator
            else {
                continue;
            };
            let arg_ty = |o: &Operand| match o {
                Operand::Copy(p) | Operand::Move(p) => place_ty(&types, p),
                Operand::Constant(_) => None,
                _ => Some(Ty::Other(String::new())),
            };
            let self_call = func.is_plain() && func.to_string() == name && args.len() == f.params.len();
            if !self_call || dest.as_local() != Some(Local(0)) || !super::cycles::goes_to_return(*target, &blocks) {
                continue;
            }
            if args.iter().any(|a| arg_ty(a).is_some_and(|t| pointer(&t))) {
                continue;
            }
            let mut temps = vec![];
            for (a, (_, ty)) in args.iter().zip(&f.params) {
                let t = Local(next);
                next += 1;
                decls.push(super::ir::Decl {
                    local: t,
                    mutable: false,
                    ty: ty.clone(),
                });
                b.statements
                    .push(Statement::Assign(Place::local(t), Rvalue::Use(a.clone())));
                temps.push(t);
            }
            for (t, (p, _)) in temps.into_iter().zip(&f.params) {
                b.statements.push(Statement::Assign(
                    Place::local(*p),
                    Rvalue::Use(Operand::Move(Place::local(t))),
                ));
            }
            b.terminator = Terminator::Goto(start);
            looped = true;
        }
        f.body.decls.extend(decls);
        if looped {
            let entry = f
                .body
                .blocks
                .iter_mut()
                .find(|b| b.name == super::ir::BasicBlock(0))
                .unwrap();
            let header = Block {
                name: start,
                statements: std::mem::take(&mut entry.statements),
                terminator: std::mem::replace(&mut entry.terminator, Terminator::Goto(start)),
            };
            f.body.blocks.push(header);
        }
    }
}

/// The type of a place, from its local's through its projections.
pub(super) fn place_ty(types: &HashMap<Local, Ty>, p: &Place) -> Option<Ty> {
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

/// A usize compared with a constant k past a card, as rustc's inlined `i16::try_from(n)` tests `n > 32767`: a usize
/// on a card is at most 999 and one past a card (held below 0, `ranges::huge_usizes`) is above 2^64 - 1000, so
/// `x > k` and `x >= k` are `x > 999`, and `x < k` and `x <= k` are `x <= 999`, for any k between, and the
/// constant needs no cell.
struct PastACardBound;

impl VisitMut for PastACardBound {
    fn visit_rvalue(&mut self, r: &mut Rvalue) {
        walk_rvalue(self, r);
        let Rvalue::BinaryOp(op @ (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge), a, b) = r else {
            return;
        };
        let between = |o: &Operand| {
            matches!(o, Operand::Constant(Const::Int { value, ty: IntTy::Usize })
                if (1000..(1i128 << 64) - 1000).contains(value))
        };
        // x op k, the constant on the right
        let (x, above) = match (between(a), between(b)) {
            (false, true) => (a.clone(), matches!(op, BinOp::Gt | BinOp::Ge)),
            (true, false) => (b.clone(), matches!(op, BinOp::Lt | BinOp::Le)),
            _ => return,
        };
        let card = Operand::Constant(Const::Int {
            value: 999,
            ty: IntTy::Usize,
        });
        *r = Rvalue::BinaryOp(if above { BinOp::Gt } else { BinOp::Le }, x, card);
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
            // a slice's length and a slice made of its parts are operations the IR writes as variants
            let operation = p.is_path(&["PtrMetadata"]) || p.is_path(&["SliceFromParts"]);
            if !ops.is_empty() && !self.names_a_variant(p) && !operation {
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

/// The functions into which rustc inlined an integer method of core's, of a type other than i32, that works in i32
/// past what a card holds, by the name `functions` gives them, each with that method's path. i32 is i16 on CARDIAC,
/// which is right for a program's own (Rust's default integer, on a card either way, and its methods, such as a
/// range's step); but core widens to i32 only to work past what the narrower type holds: `i16::midpoint` adds the
/// two in i32, so that the sum can't overflow, and on a card it can. Where every value the method's i32 locals can
/// hold is on a card (`u8::abs_diff`'s difference of two u8s, -255 to 255), i32 is as good as i16, and it compiles.
fn widened_in_core(file: &File) -> HashMap<String, Path> {
    let mut out = HashMap::new();
    let fns = functions(file);
    for (name, f) in fns.iter() {
        let i32_locals: Vec<Local> = f
            .body
            .decls
            .iter()
            .filter(|d| d.ty == Ty::Int(IntTy::I32))
            .map(|d| d.local)
            .collect();
        // `core::num::<impl i16>::midpoint`: an integer method of core's, of a type other than i32 itself
        let widens = |p: &Path| {
            matches!(p.segments.first(), Some(Segment::Name { name, .. }) if name == "core")
                && p.segments
                    .iter()
                    .any(|s| matches!(s, Segment::Impl(Ty::Int(t)) if *t != IntTy::I32))
        };
        let widened: Vec<&(Local, Path)> = f
            .body
            .inlined
            .iter()
            .filter(|(l, p)| i32_locals.contains(l) && widens(p))
            .collect();
        if widened.is_empty() {
            continue;
        }
        // all of core's i32s here, those of a scope within the method's (`<impl i32>::unsigned_abs`) among them
        let core_i32s: Vec<Local> = i32_locals
            .iter()
            .copied()
            .filter(|l| f.body.inlined.iter().any(|(m, _)| m == l))
            .collect();
        if !on_a_card(f, &core_i32s) {
            out.insert(name.clone(), widened[0].1.clone());
        }
    }
    out
}

/// The values a local can hold, from `lo` to `hi`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    lo: i128,
    hi: i128,
}

impl Span {
    const CARD: Span = Span { lo: -999, hi: 999 };
    const ANYTHING: Span = Span {
        lo: i128::MIN,
        hi: i128::MAX,
    };

    fn on_a_card(self) -> bool {
        Span::CARD.lo <= self.lo && self.hi <= Span::CARD.hi
    }

    fn join(self, o: Span) -> Span {
        Span {
            lo: self.lo.min(o.lo),
            hi: self.hi.max(o.hi),
        }
    }

    /// What a cell of the type holds: a u8 or i8 its own range, an i16 (and a program's i32) a card's; none for the
    /// rest, whose cell may stand for a number past a card (a huge usize, a u16 or u32 of `wide`).
    fn of(t: &Ty) -> Option<Span> {
        match t {
            Ty::Int(IntTy::U8) => Some(Span { lo: 0, hi: 255 }),
            Ty::Int(IntTy::I8) => Some(Span { lo: -128, hi: 127 }),
            Ty::Int(IntTy::I16 | IntTy::I32) => Some(Span::CARD),
            _ => None,
        }
    }
}

/// Whether every value each of `locals` (i32s of core's) can hold is on a card, worked out from every assignment to
/// it, over and over until nothing grows: a constant, a cast from a u8, an i8 or an i16, `+`, `-`, `*` and negation
/// of such values. A local assigned any other way, or written through a reference or a projection, may hold anything.
/// Every value inside a card, a card's arithmetic is i32's.
fn on_a_card(f: &Function, locals: &[Local]) -> bool {
    let types: HashMap<Local, &Ty> = f
        .params
        .iter()
        .map(|(l, t)| (*l, t))
        .chain(f.body.decls.iter().map(|d| (d.local, &d.ty)))
        .collect();
    let mut anything: Vec<Local> = vec![];
    let mut assigned: Vec<(Local, &Rvalue)> = vec![];
    let ours = |p: &Place| match p.base {
        PlaceBase::Local(l) if locals.contains(&l) => Some(l),
        _ => None,
    };
    for b in &f.body.blocks {
        for s in &b.statements {
            match s {
                Statement::Assign(p, r) => {
                    if let Some(l) = ours(p) {
                        if p.projection.is_empty() {
                            assigned.push((l, r));
                        } else {
                            anything.push(l);
                        }
                    }
                    if let Rvalue::Ref(_, q) = r {
                        anything.extend(ours(q));
                    }
                }
                Statement::Unsupported(u) => anything.extend(u.locals.iter().filter(|l| locals.contains(l))),
                _ => {}
            }
        }
        match &b.terminator {
            Terminator::Call { dest, .. } => anything.extend(ours(dest)),
            Terminator::Unsupported(u) => anything.extend(u.locals.iter().filter(|l| locals.contains(l))),
            _ => {}
        }
    }
    // a local that may hold anything holds what no card does, and so does every local worked out from it
    let mut spans: HashMap<Local, Span> = anything.iter().map(|l| (*l, Span::ANYTHING)).collect();
    // each round grows a span, or nothing grows and every span is known; one past a card is as far as it need go.
    // A few rounds settle the straight-line code core inlines; code that doesn't settle in 16 is refused.
    for _ in 0..16 {
        let mut grew = false;
        let mut unknown = false;
        for &(l, r) in assigned.iter().filter(|(l, _)| !anything.contains(l)) {
            // none where a local it reads has no span yet
            let Some(v) = span_of(r, &spans, locals, &types) else {
                unknown = true;
                continue;
            };
            let joined = spans.get(&l).map_or(v, |s| s.join(v));
            if spans.get(&l) != Some(&joined) {
                spans.insert(l, joined);
                grew = true;
            }
        }
        if !grew {
            // an assignment whose span is never known may hold anything, and so may its local
            return !unknown && spans.values().all(|s| s.on_a_card());
        }
        if spans.values().any(|s| !s.on_a_card()) {
            return false;
        }
    }
    false
}

/// The values `r` can have, where its operands' are known.
fn span_of(r: &Rvalue, spans: &HashMap<Local, Span>, locals: &[Local], types: &HashMap<Local, &Ty>) -> Option<Span> {
    let operand = |o: &Operand| match o {
        Operand::Constant(Const::Int { value, .. }) => Some(Span { lo: *value, hi: *value }),
        Operand::Copy(p) | Operand::Move(p) if p.projection.is_empty() => match p.base {
            PlaceBase::Local(l) if locals.contains(&l) => spans.get(&l).copied(),
            PlaceBase::Local(l) => types.get(&l).and_then(|t| Span::of(t)),
            _ => None,
        },
        _ => None,
    };
    match r {
        Rvalue::Use(o) => operand(o),
        Rvalue::Cast(o, Ty::Int(IntTy::I32), CastKind::IntToInt) => Some(operand(o).unwrap_or(Span::ANYTHING)),
        Rvalue::UnaryOp(UnOp::Neg, o) => operand(o).map(|a| Span {
            lo: a.hi.saturating_neg(),
            hi: a.lo.saturating_neg(),
        }),
        Rvalue::BinaryOp(op, a, b) => {
            let (a, b) = (operand(a)?, operand(b)?);
            match op {
                BinOp::Add | BinOp::AddUnchecked | BinOp::AddWrapping => Some(Span {
                    lo: a.lo.saturating_add(b.lo),
                    hi: a.hi.saturating_add(b.hi),
                }),
                BinOp::Sub | BinOp::SubUnchecked | BinOp::SubWrapping => Some(Span {
                    lo: a.lo.saturating_sub(b.hi),
                    hi: a.hi.saturating_sub(b.lo),
                }),
                BinOp::Mul | BinOp::MulUnchecked | BinOp::MulWrapping => {
                    let ends = [
                        a.lo.saturating_mul(b.lo),
                        a.lo.saturating_mul(b.hi),
                        a.hi.saturating_mul(b.lo),
                        a.hi.saturating_mul(b.hi),
                    ];
                    Some(Span {
                        lo: *ends.iter().min().unwrap(),
                        hi: *ends.iter().max().unwrap(),
                    })
                }
                _ => Some(Span::ANYTHING),
            }
        }
        _ => Some(Span::ANYTHING),
    }
}

/// A char is the number it is, a code point, in a cell as a usize is: never negative, compared as numbers, `as u8`
/// its low byte (mod 256). Rust has no arithmetic on chars, and every char a program can make (a literal, a `u8`
/// cast) is below 1000, a literal past a card refused as any constant past one is.
struct CharIsUsize;

impl VisitMut for CharIsUsize {
    fn visit_ty(&mut self, t: &mut Ty) {
        if *t == Ty::Char {
            *t = Ty::Int(IntTy::Usize);
        }
        walk_ty(self, t)
    }

    fn visit_const(&mut self, c: &mut Const) {
        if let Const::Char(ch) = *c {
            *c = Const::Int {
                value: ch as i128,
                ty: IntTy::Usize,
            };
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

/// An isize is an i16, as a C-like enum's discriminant already is (`enums`): what a card holds, an isize holds too,
/// and past a card it panics or keeps three digits as an i16 does. (A cast of one to a usize, or of a usize to one,
/// is then an i16's: -1 is usize::MAX, held below 0, and back.)
struct IsizeIsI16;

impl VisitMut for IsizeIsI16 {
    fn visit_ty(&mut self, t: &mut Ty) {
        if *t == Ty::Int(IntTy::Isize) {
            *t = Ty::Int(IntTy::I16);
        }
        walk_ty(self, t)
    }

    fn visit_const(&mut self, c: &mut Const) {
        if let Const::Int {
            ty: ty @ IntTy::Isize, ..
        } = c
        {
            *ty = IntTy::I16;
        }
        walk_const(self, c)
    }

    /// `isize::MAX`, which no card holds, as `i16::MAX` is refused
    fn visit_path(&mut self, p: &mut Path) {
        for s in &mut p.segments {
            if let Segment::Name { name, .. } = s {
                if name == "isize" {
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

/// The statics whose first value is a literal: a number or a bool (`static mut COUNT: i16 = 0`), or an array of them
/// (`static T: [i16; 4] = [11, 22, 33, 44]`); each by its name, as the allocations a body points into name it. A
/// static whose value rustc works out otherwise is left out, and a pointer to it is refused where it is used.
fn statics(file: &File) -> Statics {
    let scalar = |t: &Ty| matches!(t, Ty::Int(IntTy::I16 | IntTy::I8 | IntTy::U8 | IntTy::Usize) | Ty::Bool);
    let value = |o: &Operand| match o {
        Operand::Constant(Const::Int { value, .. }) => Some(*value as i64),
        Operand::Constant(Const::Bool(b)) => Some(*b as i64),
        _ => None,
    };
    let mut out = Statics::default();
    for item in &file.items {
        let Item::Static(s) = item else { continue };
        let Some(name) = s.path.single().filter(|_| s.path.is_plain()) else {
            continue;
        };
        let values = match (&s.ty, result_value(&s.body).next()) {
            (t, Some(Rvalue::Use(o))) if scalar(t) => value(o).map(|v| vec![v]),
            (Ty::Array(e, ArrayLen::Known(n)), Some(Rvalue::Array(os))) if scalar(e) && os.len() as u64 == *n => {
                os.iter().map(value).collect()
            }
            (Ty::Array(e, ArrayLen::Known(n)), Some(Rvalue::Repeat(o, k))) if scalar(e) && k == n => {
                value(o).map(|v| vec![v; *n as usize])
            }
            _ => None,
        };
        if let Some(values) = values {
            out.values.set(name.to_string(), values);
        }
    }
    for item in &file.items {
        let Item::Alloc(a) = item else { continue };
        if let Some(name) = a.of_static.as_ref().and_then(|p| p.single()) {
            if out.values.has(name) {
                out.of_alloc.insert(a.name.clone(), name.to_string());
            }
        }
    }
    out
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
            ConstValue::Const(Const::Int { .. }) => return Err(crate::codes::unsupported_const(name, ty)),
            _ if matches!(ty, Ty::Path(p) if p.is("f32") || p.is("f64")) => {
                return Err(crate::codes::unsupported_const(name, ty))
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
    let mut types: HashMap<String, Ty> = HashMap::new();
    for item in &file.items {
        let Item::Const(ConstItem {
            path,
            ty: ty @ Ty::Array(elem, ArrayLen::Known(_)),
            value: ConstValue::Body(body),
        }) = item
        else {
            continue;
        };
        let Some(name) = path.single().filter(|_| path.is_plain()) else {
            continue;
        };
        if !matches!(
            **elem,
            Ty::Int(IntTy::I16 | IntTy::I8 | IntTy::U8 | IntTy::Usize) | Ty::Bool
        ) {
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
            types.insert(name.to_string(), ty.clone());
        }
    }
    // `for x in T` casts the constant itself to make its iterator (`const T as [MaybeUninit<usize>; 8]
    // (Transmute)`): the constant goes into a local of its own on the line before, as `let t = T;` would put it,
    // and the cast is of that
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
        for b in f.body.blocks.iter_mut() {
            let mut si = 0;
            while si < b.statements.len() {
                let named = match &b.statements[si] {
                    Statement::Assign(_, Rvalue::Cast(Operand::Constant(Const::Path(p)), _, CastKind::Transmute)) => {
                        const_path(p).and_then(|n| n.last().map(|l| l.to_string()))
                    }
                    _ => None,
                };
                let Some((literal, ty)) = named.and_then(|n| Some((arrays.get(&n)?.clone(), types.get(&n)?.clone())))
                else {
                    si += 1;
                    continue;
                };
                let t = Local(next);
                next += 1;
                f.body.decls.push(super::ir::Decl {
                    local: t,
                    mutable: false,
                    ty,
                });
                if let Statement::Assign(_, Rvalue::Cast(o, _, _)) = &mut b.statements[si] {
                    *o = Operand::Copy(Place::local(t));
                }
                b.statements.insert(si, Statement::Assign(Place::local(t), literal));
                si += 2;
            }
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

/// A closure's own function, `main::{closure#0}`, by the name the compiler reads it as, `main_closure0` (a call
/// to it is `map`'s, say).
pub(super) fn closure_name(path: &ItemPath) -> Option<String> {
    let [owner, c] = path.segments.as_slice() else {
        return None;
    };
    let n = c.strip_prefix("{closure#")?.strip_suffix('}')?;
    (word(owner) && word(n) && n.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{owner}_closure{n}"))
}

/// The functions the compiler reads: those named by one plain name (not an impl's), and closures' (as
/// `OWNER_closureN`), in the order the file gives them. Of two of the same name (a const fn and its `// MIR FOR CTFE` copy), the later body is the one kept, in
/// the earlier one's place.
fn functions(file: &File) -> Ordered<String, Function> {
    let mut fns = Ordered::new();
    for item in &file.items {
        let Item::Fn(f) = item else { continue };
        let name = match f.path.segments.as_slice() {
            [owner, _] if word(owner) => match closure_name(&f.path) {
                Some(name) => name,
                None => continue,
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
                widened: None,
            },
        );
    }
    fns
}
