//! C-like enums: `enum Colour { Red, Green, Blue }` is one cell, the variant's number. rustc's MIR names a
//! variant (`const Colour::Green`) and branches on its number (`switchInt(discriminant(c)) -> [1: ..]`), but never
//! says which number a name is: the enum's definition isn't in it. The CARDIAC driver (`driver/`), which is rustc,
//! looks the definitions up (`driver/src/facts.rs`): each enum's module path, and its variants in order, each with
//! its number and how many fields it carries.
//!
//! The MIR names an enum by the shortest path that is unique in its crate (`Colour`, or `paint::Colour` when another
//! module has a `Colour` too), so a definition is found by the end of its module path. Another crate's enum
//! (`core::option::Option`) is found the same way by its whole path, and a generic one by its type arguments too.
//!
//! An enum with data (`Option<i16>`, `enum Shape { Dot, Circle(i16), Rect(i16, i16) }`) is a tag cell, the
//! variant's number, and cells for the fields, which the variants share (only one is live at a time): a field
//! takes the first cell of its type that no earlier field of its variant has. `data_enums` writes each such enum as
//! a struct of those cells, which the struct pass then takes apart as it does any struct:
//!   (_x as V).k                 the cell of V's field k
//!   discriminant(_x)            the tag
//!   _x = E::V(a, b)             the tag, then each field's cell
//!   _x = E::V, const E::V       the tag
use std::collections::{HashMap, HashSet};

use super::ir::visit_mut::{walk_const, walk_rvalue, walk_terminator, walk_ty, VisitMut};
use super::ir::Locals;
use super::ir::{
    ArrayLen, BasicBlock, BinOp, CastKind, Const, File, GenericArg, IntTy, Item, Local, Operand, Path, Place,
    Projection, Rvalue, Segment, Statement, Terminator, Ty,
};
use super::structs::{Field, Function};
use super::util::Ordered;
use super::Fail;

/// An enum as the driver wrote it down: its module path and name (`["paint", "Colour"]`, or another crate's,
/// `["core", "option", "Option"]`), its type arguments (`i16` for `Option<i16>`), and its variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDef {
    pub path: Vec<String>,
    pub args: Vec<Ty>,
    /// Is it the crate's own, its path within the crate? (Another crate's starts with that crate's name.)
    pub in_crate: bool,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    /// The variant's number (its discriminant).
    pub value: i128,
    /// The types of the fields it carries: none for a plain name.
    pub fields: Vec<Ty>,
}

impl EnumDef {
    /// No variant carries data: the enum is its number.
    pub fn c_like(&self) -> bool {
        self.variants.iter().all(|v| v.fields.is_empty())
    }
}

/// The words of a path as the MIR prints it (`paint::Colour`), or none if a segment isn't a plain word.
fn words(p: &Path) -> Option<Vec<&str>> {
    let (w, args) = words_args(p)?;
    args.is_empty().then_some(w)
}

/// The words of a path as the MIR prints it (`core::option::Option<i16>`, `Option::<i16>::Some`), and the type
/// arguments it gives. A path into std is core's (`std::result::Result`): std's enums are core's.
fn words_args(p: &Path) -> Option<(Vec<&str>, Vec<&Ty>)> {
    if p.qself.is_some() {
        return None;
    }
    let mut args = vec![];
    let mut w = vec![];
    for (i, s) in p.segments.iter().enumerate() {
        let Segment::Name { name, args: a } = s else {
            return None;
        };
        w.push(if i == 0 && name == "std" { "core" } else { name.as_str() });
        for a in a.iter().flat_map(|a| &a.args) {
            if let GenericArg::Ty(t) = a {
                args.push(t);
            }
        }
    }
    Some((w, args))
}

/// A definition's path, std's as core's (rustc names `Option` in a program with std `std::option::Option`).
fn def_path(d: &EnumDef) -> Vec<&str> {
    d.path
        .iter()
        .enumerate()
        .map(|(i, w)| if i == 0 && w == "std" { "core" } else { w.as_str() })
        .collect()
}

/// Is a definition's path `name`, or does it end with it?
fn named(d: &EnumDef, name: &[&str], ends: bool) -> bool {
    let path = def_path(d);
    if ends {
        path.ends_with(name)
    } else {
        path == name
    }
}

/// The definition the MIR's `name` (`Colour`, `paint::Colour`) is, with type arguments `args`: the one whose path
/// it is, crate-relative (the MIR's full path for an enum whose name another shares); else the one whose path it
/// ends (a trimmed path, the enum's name alone when that is unique).
fn find<'a>(defs: &'a [EnumDef], name: &[&str], args: &[&Ty]) -> Option<&'a EnumDef> {
    let given = |d: &&EnumDef| d.args.iter().eq(args.iter().copied());
    if let Some(d) = defs.iter().filter(given).find(|d| named(d, name, false)) {
        return Some(d);
    }
    let mut found = defs.iter().filter(given).filter(|d| named(d, name, true));
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// Each C-like enum's variants' numbers, by the MIR's name for the enum (`Colour`) and the variant's.
struct Numbers {
    /// enum (as the MIR names it) -> variant -> number
    of: HashMap<String, HashMap<String, i128>>,
}

impl Numbers {
    fn variant(&self, p: &Path) -> Option<i128> {
        let w = words(p)?;
        let (variant, ty) = w.split_last()?;
        self.of.get(&ty.join("::"))?.get(*variant).copied()
    }

    fn is_enum(&self, p: &Path) -> bool {
        words(p).is_some_and(|w| self.of.contains_key(&w.join("::")))
    }
}

/// The enum types the file's MIR names: each type path that a definition is, by its words.
fn enum_types(file: &File, defs: &[EnumDef]) -> Vec<Vec<String>> {
    struct Seen<'a> {
        defs: &'a [EnumDef],
        found: Vec<Vec<String>>,
    }
    impl VisitMut for Seen<'_> {
        fn visit_ty(&mut self, t: &mut Ty) {
            if let Ty::Path(p) = t {
                if let Some(w) = words(p) {
                    if find(self.defs, &w, &[]).is_some() && !self.found.iter().any(|f| f.iter().eq(&w)) {
                        self.found.push(w.iter().map(|s| s.to_string()).collect());
                    }
                }
            }
            walk_ty(self, t)
        }
    }
    let mut seen = Seen { defs, found: vec![] };
    let mut copy = file.clone();
    copy.visit(&mut seen);
    seen.found
}

/// Each C-like enum the file uses is its number: its type is i16, a variant (`const Colour::Green`) is the
/// variant's number (`discriminant(c)` is `c`, once the for loops are lowered: `data_enums`). A number past a card
/// is refused. (An enum with data is `data_enums`'.)
pub fn c_like_enums(file: &mut File, defs: &[EnumDef]) -> Result<(), Fail> {
    let mut numbers = Numbers { of: HashMap::new() };
    for name in enum_types(file, defs) {
        let words: Vec<&str> = name.iter().map(String::as_str).collect();
        let def = find(defs, &words, &[]).expect("found once already");
        let shown = name.join("::");
        // an enum with data is data_enums'; one with no variants (`Infallible`) has no values to number
        if !def.c_like() || def.variants.is_empty() {
            continue;
        }
        let mut each = HashMap::new();
        for v in &def.variants {
            if !(-999..=999).contains(&v.value) {
                return Err(format!(
                    "enum {shown}: {} is {}, past what a card holds (-999 to 999)",
                    v.name, v.value
                ));
            }
            each.insert(v.name.clone(), v.value);
        }
        numbers.of.insert(shown, each);
    }
    if numbers.of.is_empty() {
        return Ok(());
    }
    bare_variants(file, &numbers);
    discriminants_in_cells(file, &numbers);
    struct Rewrite<'a>(&'a Numbers);
    impl VisitMut for Rewrite<'_> {
        fn visit_ty(&mut self, t: &mut Ty) {
            // a discriminant is an isize: on CARDIAC the enum's number is a card, as an i32 is
            if matches!(t, Ty::Path(p) if self.0.is_enum(p)) || *t == Ty::Int(IntTy::Isize) {
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
            if let Const::Path(p) = c {
                if let Some(value) = self.0.variant(p) {
                    *c = Const::Int { value, ty: IntTy::I16 };
                }
            }
            walk_const(self, c)
        }
        // a call's path keeps the enum's name: `<Colour as PartialEq>::eq` names the impl's function, as the
        // function is named
        fn visit_terminator(&mut self, t: &mut Terminator) {
            let func = match t {
                Terminator::Call { func, .. } => Some(func.clone()),
                _ => None,
            };
            walk_terminator(self, t);
            if let (Terminator::Call { func, .. }, Some(was)) = (t, func) {
                *func = was;
            }
        }
        // a variant written as built, `_1 = Colour::Green` (as a promoted constant's body has it): its number
        fn visit_rvalue(&mut self, r: &mut Rvalue) {
            if let Rvalue::Variant(p, ops) = r {
                if let (true, Some(value)) = (ops.is_empty(), self.0.variant(p)) {
                    *r = Rvalue::Use(Operand::Constant(Const::Int { value, ty: IntTy::I16 }));
                    return;
                }
            }
            walk_rvalue(self, r)
        }
    }
    file.visit(&mut Rewrite(&numbers));
    Ok(())
}

/// A variant named alone, `const Less`, as rustc prints one in code it inlined from another crate (core's
/// `Ordering::reverse`): the variant of the enum it is set to or compared with, by that place's type.
fn bare_variants(file: &mut File, numbers: &Numbers) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let types = super::front::local_types(&f.params, &f.body.decls);
        let full = |p: &Place, c: &mut Operand| {
            let Operand::Constant(Const::Path(v)) = c else { return };
            let Some(Ty::Path(e)) = super::front::place_ty(&types, p) else {
                return;
            };
            let [Segment::Name { name, args: None }] = v.segments.as_slice() else {
                return;
            };
            let mut named = e.clone();
            named.segments.push(Segment::Name {
                name: name.clone(),
                args: None,
            });
            if v.qself.is_none() && numbers.variant(&named).is_some() {
                *v = named;
            }
        };
        for b in f.body.blocks.iter_mut() {
            for s in b.statements.iter_mut() {
                match s {
                    Statement::Assign(d, Rvalue::Use(c)) => full(d, c),
                    Statement::Assign(_, Rvalue::BinaryOp(_, x, y)) => {
                        if let Some(p) = x.place().cloned() {
                            full(&p, y);
                        }
                        if let Some(p) = y.place().cloned() {
                            full(&p, x);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// A local holding a C-like enum's discriminant in a type the backend doesn't have (an i8: `Ordering` is
/// `#[repr(i8)]`) is an i16, as the enum itself is, and so is each constant it is compared with. Only where it is
/// set by `discriminant` alone and read only to compare, switch on or cast: those give the same answer in either
/// type for every variant's number, where arithmetic in the narrow type could overflow where an i16 doesn't.
fn discriminants_in_cells(file: &mut File, numbers: &Numbers) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let types = super::front::local_types(&f.params, &f.body.decls);
        let of_enum = |p: &Place| matches!(super::front::place_ty(&types, p), Some(Ty::Path(e)) if numbers.is_enum(&e));
        let narrow: Vec<(Local, IntTy)> = f
            .body
            .decls
            .iter()
            .filter_map(|d| match d.ty {
                Ty::Int(t @ (IntTy::I8 | IntTy::U16 | IntTy::U32 | IntTy::I64 | IntTy::U64)) => Some((d.local, t)),
                _ => None,
            })
            .collect();
        let whole = |o: &Operand, l: Local| matches!(o, Operand::Copy(p) | Operand::Move(p) if p.as_local() == Some(l));
        let mut cells = vec![];
        for (l, t) in narrow {
            let mut allowed = 0;
            for b in &f.body.blocks {
                for s in &b.statements {
                    allowed += match s {
                        Statement::Assign(d, Rvalue::Discriminant(p)) if d.as_local() == Some(l) && of_enum(p) => 1,
                        Statement::Assign(_, Rvalue::BinaryOp(op, x, y)) if comparison(*op) => {
                            [x, y].iter().filter(|o| whole(o, l)).count()
                        }
                        Statement::Assign(_, Rvalue::Cast(o, _, CastKind::IntToInt)) if whole(o, l) => 1,
                        Statement::StorageLive(x) | Statement::StorageDead(x) if *x == l => 1,
                        _ => 0,
                    };
                }
                if let Terminator::SwitchInt { discr, .. } = &b.terminator {
                    allowed += whole(discr, l) as usize;
                }
            }
            if allowed == super::ir::occurrences(&f.body.blocks, l) {
                cells.push((l, t));
            }
        }
        for (l, t) in &cells {
            for d in f.body.decls.iter_mut().filter(|d| d.local == *l) {
                d.ty = Ty::Int(IntTy::I16);
            }
            for b in f.body.blocks.iter_mut() {
                for s in b.statements.iter_mut() {
                    let Statement::Assign(_, Rvalue::BinaryOp(op, x, y)) = s else {
                        continue;
                    };
                    if !comparison(*op) || !(whole(x, *l) || whole(y, *l)) {
                        continue;
                    }
                    for o in [x, y] {
                        if let Operand::Constant(Const::Int { ty, .. }) = o {
                            if ty == t {
                                *ty = IntTy::I16;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

/// What a variant's field takes: a cell of its type, the cells of an enum with data (its layout's, by index), or
/// none, for a type with no values (`Infallible`, in what `?` builds) or with one (`()`, in `Option<()>`).
#[derive(Clone, Debug)]
enum Kind {
    Cell(Ty),
    Enum(usize),
    Never,
    Unit,
}

/// A variant in its enum's cells: its number, and where each of its fields starts and what it takes.
type Cells = (i128, Vec<(u32, Kind)>);

/// An enum with data in cells: the struct it is written as (its name, and its cells, the tag first), and for each
/// variant, by name, its number and where each of its fields starts and what it takes.
struct Layout {
    strukt: String,
    fields: Vec<Field>,
    variants: HashMap<String, Cells>,
    /// No tag cell: `None` is -1 in the payload's cell (see `Niche`).
    niche: Option<Niche>,
    /// Two variants numbered 0 and 1, with a tag cell: the tag holds the number less one, -1 or 0, so which it is is
    /// the tag's sign, one TAC (a `match` on a number is a subtraction and a TAC or two).
    sign: bool,
}

/// The niche layout of an enum shaped as `Option<i16>` (a plain variant and one with a single `i16`): no
/// tag cell, the plain variant is -1 in the payload's cell, so telling them apart is a TAC. Sound only if no
/// payload is ever negative: each one written is followed by a guard, `assert(payload >= 0, "niche ..")`, and the
/// layout is kept only if the range pass proves every guard, which then costs nothing (`compile_mir`).
#[derive(Clone, Copy, Debug)]
struct Niche {
    /// the plain variant's number, and the other's
    none: i128,
    some: i128,
}

/// The message of a niche's guard, which names the struct the enum is written as.
pub const NICHE_GUARD: &str = "niche guard: ";

/// In `no_niche`: no enum has a niche.
pub const NO_NICHES: &str = "*";

impl Layout {
    fn ty(&self) -> Ty {
        Ty::Path(Path {
            qself: None,
            segments: vec![Segment::Name {
                name: self.strukt.clone(),
                args: None,
            }],
        })
    }
}

/// Are two types the same, as the MIR may print one with its paths in full and once trimmed
/// (`core::convert::Infallible`, `Infallible`)? Paths compare by their last word and their type arguments.
fn same(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        (Ty::Path(p), Ty::Path(q)) => match (words_args(p), words_args(q)) {
            (Some((w, x)), Some((v, y))) => {
                w.last() == v.last() && x.len() == y.len() && x.iter().zip(&y).all(|(a, b)| same(a, b))
            }
            _ => p == q,
        },
        (Ty::Array(a, n), Ty::Array(b, m)) => n == m && same(a, b),
        (Ty::Slice(a), Ty::Slice(b)) => same(a, b),
        (Ty::Ref { to: a, mutable: m, .. }, Ty::Ref { to: b, mutable: n, .. }) => m == n && same(a, b),
        (Ty::Tuple(a), Ty::Tuple(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b)),
        _ => a == b,
    }
}

/// The definition an enum type's path is (`core::option::Option<i16>`, `Option<i16>`, `Shape`).
/// Does the path name a variant of one of the enums, `Result::<i16, i16>::Err`? (`Flag`, a tuple struct, doesn't.)
pub fn names_a_variant(defs: &[EnumDef], p: &Path) -> bool {
    let Some((Segment::Name { name, .. }, ty)) = p.segments.split_last() else {
        return false;
    };
    let ty = Path {
        qself: p.qself.clone(),
        segments: ty.to_vec(),
    };
    !ty.segments.is_empty() && def_of(defs, &ty).is_some_and(|(_, d)| d.variants.iter().any(|v| &v.name == name))
}

fn def_of<'a>(defs: &'a [EnumDef], p: &Path) -> Option<(usize, &'a EnumDef)> {
    let (w, args) = words_args(p)?;
    let given = |d: &EnumDef| d.args.len() == args.len() && d.args.iter().zip(&args).all(|(a, b)| same(a, b));
    let exact = defs.iter().position(|d| given(d) && named(d, &w, false));
    let i = exact.or_else(|| {
        let mut found = defs
            .iter()
            .enumerate()
            .filter(|(_, d)| given(d) && named(d, &w, true))
            .map(|(i, _)| i);
        let first = found.next()?;
        found.next().is_none().then_some(first)
    })?;
    Some((i, &defs[i]))
}

/// The definitions as the IR names their types once the front end has run: an i32 type argument is an i16, as is a
/// C-like enum; each once.
pub(crate) fn normalized(defs: &[EnumDef]) -> Vec<EnumDef> {
    let mut defs = defs.to_vec();
    for i in 0..defs.len() {
        let none = HashSet::new();
        let e = Enums {
            defs: &defs,
            layouts: vec![],
            no_niche: &none,
        };
        let norm = |t: &Ty| e.scalar(t).unwrap_or_else(|| t.clone());
        let args: Vec<Ty> = defs[i].args.iter().map(norm).collect();
        defs[i].args = args;
    }
    // one of each: `Option<i32>`, from the cardiac crate say, is the program's `Option<i16>`
    let mut one = vec![];
    for d in defs {
        if !one.iter().any(|e: &EnumDef| e.path == d.path && e.args == d.args) {
            one.push(d);
        }
    }
    one
}

/// The number of variant `name` of the enum type `t` (`core::option::Option<i16>`), from `defs` as `normalized`
/// gives them.
pub(crate) fn variant_number(defs: &[EnumDef], t: &Ty, name: &str) -> Option<i128> {
    let Ty::Path(p) = t else { return None };
    let (_, d) = def_of(defs, p)?;
    d.variants.iter().find(|v| v.name == name).map(|v| v.value)
}

/// The enums with data the functions use, laid out.
struct Enums<'a> {
    defs: &'a [EnumDef],
    /// each with its definition's index in `defs`
    layouts: Vec<(usize, Layout)>,
    /// the enums (by the struct each is written as) whose niche the range pass didn't prove: each with a tag cell
    no_niche: &'a HashSet<String>,
}

impl Enums<'_> {
    /// A field's type as a cell: an i32 is an i16, as is a C-like enum.
    fn scalar(&self, t: &Ty) -> Option<Ty> {
        match t {
            Ty::Int(IntTy::I32) => Some(Ty::Int(IntTy::I16)),
            Ty::Int(IntTy::I16 | IntTy::U8 | IntTy::Usize) | Ty::Bool => Some(t.clone()),
            // (a char is a usize once the front end has run: `front::CharIsUsize`)
            Ty::Char => Some(Ty::Int(IntTy::Usize)),
            Ty::Path(p) => def_of(self.defs, p)
                .filter(|(_, d)| d.c_like() && !d.variants.is_empty())
                .map(|_| Ty::Int(IntTy::I16)),
            _ => None,
        }
    }

    /// The layout of definition `i`, laid out now if it isn't yet: its index in `layouts`.
    fn lay_out(&mut self, i: usize) -> Result<usize, Fail> {
        if let Some(at) = self.layouts.iter().position(|(d, _)| *d == i) {
            return Ok(at);
        }
        let def = &self.defs[i];
        let shown = def.path.join("::");
        let mut fields = vec![Field {
            name: "tag".into(),
            ty: Ty::Int(IntTy::I16),
        }];
        let mut variants = HashMap::new();
        for v in &def.variants {
            if !(-999..=999).contains(&v.value) {
                return Err(format!(
                    "enum {shown}: {} is {}, past what a card holds (-999 to 999)",
                    v.name, v.value
                ));
            }
            let mut taken: Vec<u32> = vec![];
            let mut at = vec![];
            for (k, t) in v.fields.iter().enumerate() {
                let kind = self.kind(t).map_err(|e| {
                    e.unwrap_or_else(|| {
                        format!(
                            "enum {shown}: field {k} of {} is {t}: the fields of an enum's variants must be scalars (i16, u8, usize, bool), enums, structs or arrays of scalars for now",
                            v.name
                        )
                    })
                })?;
                let cells: Vec<Field> = match &kind {
                    Kind::Cell(t) => vec![Field {
                        name: format!("{}_{k}", v.name),
                        ty: t.clone(),
                    }],
                    Kind::Enum(j) => self.layouts[*j]
                        .1
                        .fields
                        .iter()
                        .map(|f| Field {
                            name: format!("{}_{k}_{}", v.name, f.name),
                            ty: f.ty.clone(),
                        })
                        .collect(),
                    Kind::Never | Kind::Unit => vec![],
                };
                let n = cells.len() as u32;
                // the first run of cells of these types that no earlier field of the variant has, else new ones
                let free = (1..=fields.len() as u32).find(|&s| {
                    (0..n).all(|j| {
                        fields
                            .get((s + j) as usize)
                            .is_some_and(|f| f.ty == cells[j as usize].ty)
                            && !taken.contains(&(s + j))
                    })
                });
                let start = free.unwrap_or_else(|| {
                    let s = fields.len() as u32;
                    fields.extend(cells);
                    s
                });
                taken.extend(start..start + n);
                at.push((start, kind));
            }
            variants.insert(v.name.clone(), (v.value, at));
        }
        let strukt = format!("{}__enum{i}", def.path.last().map_or("", String::as_str));
        // shaped as Option<i16>: the payload's cell alone, the plain variant -1 in it
        let mut niche = None;
        if let [a, b] = def.variants.as_slice() {
            let (none, some) = if a.fields.is_empty() { (a, b) } else { (b, a) };
            let payload = match variants.get(&some.name) {
                Some((_, at)) => match at.as_slice() {
                    // (not a u8's: -1 isn't one, and the range pass knows a u8 is 0 to 255)
                    [(_, Kind::Cell(t @ Ty::Int(IntTy::I16)))] => Some(t.clone()),
                    _ => None,
                },
                None => None,
            };
            let numbers = (none.value, some.value) == (0, 1) || (none.value, some.value) == (1, 0);
            let shaped = none.fields.is_empty() && numbers;
            let allowed = !self.no_niche.contains(&strukt) && !self.no_niche.contains(NO_NICHES);
            if let (true, Some(t)) = (shaped && allowed, payload) {
                fields = vec![Field {
                    name: format!("{}_0", some.name),
                    ty: t.clone(),
                }];
                variants.insert(none.name.clone(), (none.value, vec![]));
                variants.insert(some.name.clone(), (some.value, vec![(0, Kind::Cell(t))]));
                niche = Some(Niche {
                    none: none.value,
                    some: some.value,
                });
            }
        }
        let numbers: Vec<i128> = def.variants.iter().map(|v| v.value).collect();
        let sign = niche.is_none() && (numbers == [0, 1] || numbers == [1, 0]);
        self.layouts.push((
            i,
            Layout {
                strukt,
                fields,
                variants,
                niche,
                sign,
            },
        ));
        Ok(self.layouts.len() - 1)
    }

    /// What a field of type `t` takes. Err(None): a type no cell holds. A struct, or an array of scalars, is one
    /// field of the enum's struct, which the struct pass takes apart in turn.
    fn kind(&mut self, t: &Ty) -> Result<Kind, Option<Fail>> {
        if let Some(t) = self.scalar(t) {
            return Ok(Kind::Cell(t));
        }
        if *t == Ty::Unit {
            return Ok(Kind::Unit);
        }
        if let Ty::Array(e, n @ ArrayLen::Known(_)) = t {
            let e = self.scalar(e).ok_or(None)?;
            return Ok(Kind::Cell(Ty::Array(Box::new(e), n.clone())));
        }
        // a reference or a raw pointer to a card's value: its address, one cell (`Option<&i16>`, what a slice's
        // iterator returns)
        if let Ty::Ref { to, .. } | Ty::Ptr { to, .. } = t {
            if matches!(**to, Ty::Int(IntTy::I16 | IntTy::U8)) {
                return Ok(Kind::Cell(t.clone()));
            }
        }
        // a tuple of scalars, or of such tuples: a struct the struct pass makes of it (`structs::tuple_struct`)
        if matches!(t, Ty::Tuple(_)) && super::structs::tuple_struct(t, &mut vec![]).is_some() {
            return Ok(Kind::Cell(t.clone()));
        }
        let Ty::Path(p) = t else { return Err(None) };
        let Some((i, d)) = def_of(self.defs, p) else {
            return Ok(Kind::Cell(t.clone()));
        };
        if d.variants.is_empty() {
            return Ok(Kind::Never);
        }
        self.lay_out(i).map(Kind::Enum).map_err(Some)
    }

    /// The layout (its index) of the enum with data that a type is, if it is one.
    fn of(&self, t: &Ty) -> Option<usize> {
        let Ty::Path(p) = t else { return None };
        let (i, _) = def_of(self.defs, p)?;
        self.layouts.iter().position(|(d, _)| *d == i)
    }

    /// The layout and the variant that a variant's path (`Option::<i16>::Some`, `Shape::Dot`) names.
    fn variant(&self, p: &Path) -> Option<(usize, &Cells)> {
        let (last, ty) = p.segments.split_last()?;
        let Segment::Name { name, args: None } = last else {
            return None;
        };
        let l = self.of(&Ty::Path(Path {
            qself: p.qself.clone(),
            segments: ty.to_vec(),
        }))?;
        Some((l, self.layouts[l].1.variants.get(name)?))
    }

    fn layout(&self, l: usize) -> &Layout {
        &self.layouts[l].1
    }

    /// The type with each enum with data in it as the struct it is written as.
    fn retype(&self, t: &mut Ty) {
        if let Some(l) = self.of(t) {
            *t = self.layout(l).ty();
            return;
        }
        match t {
            Ty::Tuple(ts) => ts.iter_mut().for_each(|t| self.retype(t)),
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref { to: t, .. } | Ty::Ptr { to: t, .. } => self.retype(t),
            // a discriminant is an isize: on CARDIAC the tag is a card
            Ty::Int(IntTy::Isize) => *t = Ty::Int(IntTy::I16),
            _ => {}
        }
    }
}

/// What a place is, once its enums with data are cells.
#[derive(Clone, Debug)]
enum Resolved {
    /// No enum with data in it: as it is.
    Plain,
    /// One cell (a variant's field, or within one).
    Cell(Place),
    /// A whole enum with data: the place of the enum it is in (a local, an element of an array), where its cells
    /// start in that one's, and its layout.
    Whole(Place, u32, usize),
}

/// Each enum with data the functions use, written as a struct of its cells (see the module's comment), for the
/// struct pass to take apart: returns those structs, each by its name with its cells. Runs after the for loops are
/// lowered, which take the `Option`s of their iterators apart themselves. `no_niche`: the enums (by their structs'
/// names) to give a tag cell though shaped as `Option<i16>`, their niche unproved.
pub fn data_enums(
    fns: &mut Ordered<String, Function>,
    defs: &[EnumDef],
    no_niche: &HashSet<String>,
) -> Result<Vec<(String, Vec<Field>)>, Fail> {
    let names: Vec<String> = fns.keys().cloned().collect();
    for n in &names {
        numbers_discriminants(fns.get_mut(n).unwrap());
    }
    let defs = normalized(defs);
    let mut enums = Enums {
        defs: &defs,
        layouts: vec![],
        no_niche,
    };
    // the enums with data the functions' locals are, or have in them
    fn uses(t: &Ty, enums: &mut Enums) -> Result<(), Fail> {
        match t {
            Ty::Path(p) => {
                if let Some((i, d)) = def_of(enums.defs, p) {
                    if !d.c_like() {
                        enums.lay_out(i)?;
                    }
                }
            }
            Ty::Tuple(ts) => ts.iter().try_for_each(|t| uses(t, enums))?,
            Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref { to: t, .. } | Ty::Ptr { to: t, .. } => uses(t, enums)?,
            _ => {}
        }
        Ok(())
    }
    for (_, f) in fns.iter() {
        let tys = f
            .params
            .iter()
            .map(|(_, t)| t)
            .chain(f.body.decls.iter().map(|d| &d.ty));
        for t in tys.chain([&f.ret]) {
            uses(t, &mut enums)?;
        }
        // and a struct's fields, as the places that read them have their types (`(_4.1: Option<i16>)`)
        for b in &f.body.blocks {
            let places = b
                .statements
                .iter()
                .flat_map(|s| s.places())
                .chain(b.terminator.places());
            for x in places.flat_map(|p| &p.projection) {
                if let Projection::Field(_, t) = x {
                    uses(t, &mut enums)?;
                }
            }
        }
    }
    if enums.layouts.is_empty() {
        return Ok(vec![]);
    }
    let names: Vec<String> = fns.keys().cloned().collect();
    for fname in names {
        let f = fns.get_mut(&fname).unwrap();
        forward_refs(f, &enums);
        rewrite(&fname, f, &enums)?;
    }
    Ok(enums.layouts.into_iter().map(|(_, l)| (l.strukt, l.fields)).collect())
}

/// `discriminant(c)` of a C-like enum, which is its number already (an i16, `c_like_enums`), is `c`. Done here, after
/// the for loops are lowered, and only where the place is a number: the `Option` a range's `next()` returns is
/// switched on by its discriminant too, which the for-loop pass looks for.
fn numbers_discriminants(f: &mut Function) {
    let types = local_types(f);
    let number = |p: &Place| {
        let mut t = types.get(&p.base_local()?)?.clone();
        for x in &p.projection {
            t = match (x, t) {
                (Projection::Field(_, ft), _) => ft.clone(),
                (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
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
        matches!(t, Ty::Int(_)).then_some(())
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            if let Statement::Assign(_, r @ Rvalue::Discriminant(_)) = s {
                let Rvalue::Discriminant(p) = &*r else { unreachable!() };
                if number(p).is_some() {
                    *r = Rvalue::Use(Operand::Copy(p.clone()));
                }
            }
        }
    }
}

/// The types of a function's locals.
fn local_types(f: &Function) -> HashMap<Local, Ty> {
    let mut types: HashMap<Local, Ty> = f.params.iter().cloned().collect();
    types.extend(f.body.decls.iter().map(|d| (d.local, d.ty.clone())));
    types.insert(Local(0), f.ret.clone());
    types
}

/// A reference to an enum with data, set once to borrow a place with no index in it (`_5 = &_6`, as rustc writes
/// before calling `is_some` at opt-level 1): its uses, `(*_5)`, are that place's. A borrowed place can't change
/// while the borrow lives, but through it, so the two are one. The reference goes, if nothing else reads it.
fn forward_refs(f: &mut Function, enums: &Enums) {
    let types = local_types(f);
    let mut sets: HashMap<Local, usize> = HashMap::new();
    let mut borrows: HashMap<Local, Place> = HashMap::new();
    for b in &f.body.blocks {
        for s in &b.statements {
            if let Statement::Assign(d, rv) = s {
                if let Some(r) = d.as_local() {
                    *sets.entry(r).or_default() += 1;
                    if let Rvalue::Ref(_, p) = rv {
                        let plain =
                            p.base_local().is_some() && p.projection.iter().all(|x| matches!(x, Projection::Field(..)));
                        if plain {
                            borrows.insert(r, p.clone());
                        }
                    }
                }
            }
        }
        if let Terminator::Call { dest, .. } = &b.terminator {
            if let Some(r) = dest.as_local() {
                *sets.entry(r).or_default() += 2;
            }
        }
    }
    borrows.retain(|r, _| {
        sets.get(r) == Some(&1)
            && matches!(types.get(r), Some(Ty::Ref { to, .. }) if enums.of(to).is_some())
            && !f.params.iter().any(|(p, _)| p == r)
    });
    if borrows.is_empty() {
        return;
    }
    let mut through = |p: &mut Place| {
        if let Some(to) = p.base_local().and_then(|l| borrows.get(&l)) {
            if matches!(p.projection.first(), Some(Projection::Deref)) {
                let mut q = to.clone();
                q.projection.extend(p.projection.drain(1..));
                *p = q;
            }
        }
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            if !matches!(s, Statement::Assign(d, Rvalue::Ref(..)) if d.as_local().is_some_and(|l| borrows.contains_key(&l)))
            {
                s.places_mut(&mut through);
            }
        }
        b.terminator.places_mut(&mut through);
    }
    // the references nothing reads now
    let mut read: Vec<Local> = vec![];
    for b in &f.body.blocks {
        for s in &b.statements {
            let sets_ref = matches!(s, Statement::Assign(d, Rvalue::Ref(..)) if d.as_local().is_some_and(|l| borrows.contains_key(&l)));
            if !sets_ref {
                s.visit_locals(&mut |l| read.push(l));
            }
        }
        b.terminator.visit_locals(&mut |l| read.push(l));
    }
    let gone: Vec<Local> = borrows.keys().copied().filter(|r| !read.contains(r)).collect();
    for b in f.body.blocks.iter_mut() {
        b.statements.retain(|s| {
            !matches!(s, Statement::Assign(d, Rvalue::Ref(..)) if d.as_local().is_some_and(|l| gone.contains(&l)))
                && !matches!(s, Statement::StorageLive(l) | Statement::StorageDead(l) if gone.contains(l))
        });
    }
    f.body.decls.retain(|d| !gone.contains(&d.local));
    f.body
        .debug
        .retain(|d| !d.named_local().is_some_and(|(_, l)| gone.contains(&l)));
}

/// One function's enums with data as structs of their cells.
fn rewrite(fname: &str, f: &mut Function, enums: &Enums) -> Result<(), Fail> {
    let types = local_types(f);
    let resolve = |p: &Place| resolve(p, &types, enums);
    // cell k of a whole enum
    let cell = |whole: &Place, at: u32, k: u32, l: usize| {
        let mut p = whole.clone();
        p.projection
            .push(Projection::Field(at + k, enums.layout(l).fields[k as usize].ty.clone()));
        p
    };
    let tag = |value: i128| Rvalue::Use(Operand::Constant(Const::Int { value, ty: IntTy::I16 }));
    let next = std::cell::Cell::new(types.keys().map(|l| l.0).max().unwrap_or(0) + 1);
    let temps: std::cell::RefCell<Vec<(Local, Ty)>> = Default::default();
    let fresh = |ty: Ty| {
        let t = Local(next.get());
        next.set(t.0 + 1);
        temps.borrow_mut().push((t, ty));
        t
    };
    // the payload's cell compared with 0, as its type has it: a niche's test
    let sign = |op: BinOp, from: &Place, at: u32, l: usize| {
        let c = cell(from, at, 0, l);
        let ty = match &enums.layout(l).fields[0].ty {
            Ty::Int(t) => *t,
            _ => IntTy::I16,
        };
        Rvalue::BinaryOp(op, Operand::Copy(c), Operand::Constant(Const::Int { value: 0, ty }))
    };
    // the variant an enum is, set: its tag; with a niche, -1 in the payload's cell for the plain variant, nothing for
    // the other (its payload is written next)
    let set_tag = |to: &Place, at: u32, l: usize, value: i128| -> Vec<Statement> {
        match enums.layout(l).niche {
            None if enums.layout(l).sign => vec![Statement::Assign(cell(to, at, 0, l), tag(value - 1))],
            None => vec![Statement::Assign(cell(to, at, 0, l), tag(value))],
            Some(n) if value == n.none => {
                let ty = match &enums.layout(l).fields[0].ty {
                    Ty::Int(t) => *t,
                    _ => IntTy::I16,
                };
                vec![Statement::Assign(
                    cell(to, at, 0, l),
                    Rvalue::Use(Operand::Constant(Const::Int { value: -1, ty })),
                )]
            }
            Some(_) => vec![],
        }
    };
    // which variant an enum is: its tag; with a niche, whether the payload's cell is 0 or more
    let get_tag = |from: &Place, at: u32, l: usize| -> Rvalue {
        match enums.layout(l).niche {
            None if enums.layout(l).sign => sign(BinOp::Ge, from, at, l),
            None => Rvalue::Use(Operand::Copy(cell(from, at, 0, l))),
            Some(n) => sign(if n.some == 1 { BinOp::Ge } else { BinOp::Lt }, from, at, l),
        }
    };
    // after a payload written into a niche, its guard: the payload is 0 or more (a mark, where the block is cut for
    // the assert)
    let guard = |to: &Place, at: u32, l: usize| -> Vec<Statement> {
        if enums.layout(l).niche.is_none() {
            return vec![];
        }
        let b = fresh(Ty::Bool);
        vec![
            Statement::Assign(Place::local(b), sign(BinOp::Ge, to, at, l)),
            Statement::Unsupported(super::ir::Unsupported {
                text: format!("{NICHE_GUARD}{}", enums.layout(l).strukt),
                locals: vec![b],
            }),
        ]
    };
    // one whole enum copied into another, cell by cell (cells no variant of it has set are copied too: harmless)
    let copy_all = |to: (&Place, u32), from: (&Place, u32), l: usize, moved: bool| -> Vec<Statement> {
        (0..enums.layout(l).fields.len() as u32)
            .map(|k| {
                let src = cell(from.0, from.1, k, l);
                Statement::Assign(
                    cell(to.0, to.1, k, l),
                    Rvalue::Use(if moved { Operand::Move(src) } else { Operand::Copy(src) }),
                )
            })
            .collect()
    };
    // an operand written into a whole enum
    let write_op =
        |to: &Place, at: u32, l: usize, o: &Operand, line: &dyn std::fmt::Display| -> Result<Vec<Statement>, Fail> {
            match o {
                Operand::Constant(Const::Path(p)) => match enums.variant(p) {
                    Some((vl, (value, at_k)))
                        if vl == l && at_k.iter().all(|(_, k)| matches!(k, Kind::Never | Kind::Unit)) =>
                    {
                        Ok(set_tag(to, at, l, *value))
                    }
                    _ => Err(format!(
                        "{fname}: {line}: a constant of an enum with data is not supported yet"
                    )),
                },
                Operand::Copy(p) | Operand::Move(p) => match resolve(p)? {
                    Resolved::Whole(from, from_at, fl) if fl == l => {
                        Ok(copy_all((to, at), (&from, from_at), l, matches!(o, Operand::Move(_))))
                    }
                    _ => Err(format!(
                        "{fname}: {line}: an enum with data used this way is not supported yet"
                    )),
                },
                _ => Err(format!(
                    "{fname}: {line}: a constant of an enum with data is not supported yet"
                )),
            }
        };
    // a value written into a whole enum: a variant built, a constant variant, or another enum's cells
    let write =
        |to: &Place, at: u32, l: usize, rv: &Rvalue, line: &dyn std::fmt::Display| -> Result<Vec<Statement>, Fail> {
            let mut out = vec![];
            match rv {
                Rvalue::Variant(p, ops) => {
                    let Some((vl, (value, at_k))) = enums.variant(p).filter(|(vl, _)| *vl == l) else {
                        return Err(format!("{fname}: {line}: a variant of another enum"));
                    };
                    out.extend(set_tag(to, at, vl, *value));
                    for (o, (start, kind)) in ops.iter().zip(at_k) {
                        match kind {
                            Kind::Cell(t) => {
                                let mut d = to.clone();
                                d.projection.push(Projection::Field(at + start, t.clone()));
                                out.push(Statement::Assign(d, Rvalue::Use(o.clone())));
                            }
                            Kind::Enum(j) => out.extend(write_op(to, at + start, *j, o, line)?),
                            Kind::Never | Kind::Unit => {}
                        }
                    }
                    if !at_k.is_empty() {
                        out.extend(guard(to, at, vl));
                    }
                }
                Rvalue::Use(o) => out.extend(write_op(to, at, l, o, line)?),
                _ => {
                    return Err(format!(
                        "{fname}: {line}: an enum with data used this way is not supported yet"
                    ))
                }
            }
            Ok(out)
        };
    // a variant's constant, `const Result::<i16, i16>::Err(-9_i16)`, is written as the variant built of its values
    let built = |rv: &Rvalue| match rv {
        Rvalue::Use(Operand::Constant(Const::Variant(p, cs))) => {
            Rvalue::Variant(p.clone(), cs.iter().cloned().map(Operand::Constant).collect())
        }
        // a variant with named fields built, `Shape::Rect { w: copy _2, h: copy _3 }`: the same, its fields in the
        // order they are declared (as rustc gives them, and as the cells are laid out)
        Rvalue::Struct(p, fs) if enums.variant(p).is_some() => {
            Rvalue::Variant(p.clone(), fs.iter().map(|(_, o)| o.clone()).collect())
        }
        rv => rv.clone(),
    };
    let mut sign_reads = vec![];
    for b in f.body.blocks.iter_mut() {
        let mut statements = vec![];
        for mut s in std::mem::take(&mut b.statements) {
            // a struct's field that is a constant variant (`Card { n: 1, kind: const Option::<i16>::None }`): a whole
            // enum of its own, set before, which the struct pass copies cell by cell
            if let Statement::Assign(_, Rvalue::Struct(_, fs)) = &mut s {
                for (_, a) in fs.iter_mut() {
                    let Operand::Constant(Const::Path(p) | Const::Variant(p, _)) = &*a else {
                        continue;
                    };
                    let Some((l, _)) = enums.variant(p) else { continue };
                    let t = fresh(enums.layout(l).ty());
                    let rv = built(&Rvalue::Use(a.clone()));
                    statements.extend(write(&Place::local(t), 0, l, &rv, &*a)?);
                    *a = Operand::Move(Place::local(t));
                }
            }
            match &s {
                Statement::Assign(d, rv) => match (resolve(d)?, rv) {
                    (Resolved::Whole(to, at, l), _) => statements.extend(write(&to, at, l, &built(rv), &s)?),
                    (_, Rvalue::Discriminant(p)) => match resolve(p)? {
                        Resolved::Whole(from, at, l) => {
                            let lay = enums.layout(l);
                            if let (true, Some(dl)) = (lay.niche.is_some() || lay.sign, d.as_local()) {
                                sign_reads.push((dl, lay.sign));
                            }
                            statements.push(Statement::Assign(d.clone(), get_tag(&from, at, l)))
                        }
                        _ => statements.push(s),
                    },
                    _ => statements.push(s),
                },
                // a borrow's end: the enum's cells are plain cells, which need none
                Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop => statements.push(s),
                _ => statements.push(s),
            }
        }
        // a call's argument that is a constant variant, or an enum within another: a whole enum of its own, set
        // before the call, which the struct pass passes cell by cell
        if let Terminator::Call { args, dest, .. } = &mut b.terminator {
            for a in args.iter_mut() {
                let l = match &*a {
                    Operand::Constant(Const::Path(p) | Const::Variant(p, _)) => enums.variant(p).map(|(l, _)| l),
                    Operand::Copy(p) | Operand::Move(p) => match resolve(p)? {
                        Resolved::Whole(w, at, l) if at != 0 || !w.projection.is_empty() => Some(l),
                        _ => None,
                    },
                    _ => None,
                };
                let Some(l) = l else { continue };
                let t = fresh(enums.layout(l).ty());
                let rv = built(&Rvalue::Use(a.clone()));
                statements.extend(write(&Place::local(t), 0, l, &rv, &*a)?);
                *a = Operand::Move(Place::local(t));
            }
            if let Resolved::Whole(w, at, _) = resolve(dest)? {
                if at != 0 || !w.projection.is_empty() {
                    return Err(format!(
                        "{fname}: {}: an enum with data returned into this place is not supported yet",
                        b.terminator
                    ));
                }
            }
        }
        b.statements = statements;
    }
    // each variant's field read, `((_x as V).k: T)`, is its cell, `(_x.c: T)`; a whole enum left is one the struct
    // pass takes apart, in a local, an element, a struct's field or behind a `&mut`, but not within another enum
    let mut bad = None;
    let mut cells = |p: &mut Place| match resolve(p) {
        Ok(Resolved::Cell(c)) => *p = c,
        Ok(Resolved::Whole(w, at, _))
            if at != 0 || w.projection.iter().any(|x| matches!(x, Projection::Downcast(_))) =>
        {
            bad.get_or_insert_with(|| p.to_string());
        }
        Err(e) => {
            bad.get_or_insert(e);
        }
        _ => {}
    };
    for b in f.body.blocks.iter_mut() {
        for s in b.statements.iter_mut() {
            s.places_mut(&mut cells);
        }
        b.terminator.places_mut(&mut cells);
    }
    if let Some(p) = bad {
        return Err(format!(
            "{fname}: {p}: an enum with data used this way is not supported yet"
        ));
    }
    // each guard's mark cuts its block: the assert, and the rest of the block after it
    let mut nb = 700000;
    let mut blocks = vec![];
    for b in std::mem::take(&mut f.body.blocks) {
        let mut name = b.name;
        let mut statements = vec![];
        for s in b.statements {
            if let Statement::Unsupported(u) = &s {
                if u.text.starts_with(NICHE_GUARD) {
                    let after = BasicBlock(nb);
                    nb += 1;
                    blocks.push(super::ir::Block {
                        name,
                        statements: std::mem::take(&mut statements),
                        terminator: Terminator::Assert {
                            cond: Operand::Move(Place::local(u.locals[0])),
                            expected: true,
                            msg: u.text.clone(),
                            msg_args: vec![],
                            target: after,
                            unwind: super::ir::Unwind::Continue,
                        },
                    });
                    name = after;
                    continue;
                }
            }
            statements.push(s);
        }
        blocks.push(super::ir::Block {
            name,
            statements,
            terminator: b.terminator,
        });
    }
    f.body.blocks = blocks;
    // a niche's or a sign tag's discriminant only switched on is the bool the cell's sign test makes (0 or 1, as
    // the variants' numbers are), so the test and the switch are one TAC: its switch's arms, a bool's
    // `x.is_none()` at opt-level 2 is `d == 1` negated, d the discriminant: with a niche or a sign tag, d is the
    // payload's (or tag's) sign test, and so are `d == k`, `d != k` and their negations, each a test of the cell's
    // sign (one TAC once switched on) rather than a 0 or 1 worked out and compared again
    {
        let negate = |op: BinOp| if op == BinOp::Ge { BinOp::Lt } else { BinOp::Ge };
        // within a block, in order: each local known to hold a sign test, dropped once the cell it reads (or the local
        // itself) is written again
        for b in f.body.blocks.iter_mut() {
            let mut tests: HashMap<Local, (BinOp, Operand, Operand)> = HashMap::new();
            for st in b.statements.iter_mut() {
                let Statement::Assign(y, rv) = st else { continue };
                let written = y.base_local();
                let mut known = None;
                match rv {
                    Rvalue::BinaryOp(op @ (BinOp::Ge | BinOp::Lt), a, k)
                        if y.as_local().is_some_and(|d| sign_reads.iter().any(|(r, _)| *r == d)) =>
                    {
                        known = Some((*op, a.clone(), k.clone()));
                    }
                    Rvalue::BinaryOp(
                        op @ (BinOp::Eq | BinOp::Ne),
                        Operand::Copy(p) | Operand::Move(p),
                        Operand::Constant(c),
                    ) => {
                        let k = match c {
                            Const::Int { value, .. } => Some(*value),
                            Const::Bool(b) => Some(*b as i128),
                            _ => None,
                        };
                        if let (Some(k @ 0..=1), Some((t, a, z))) =
                            (k, p.as_local().and_then(|d| tests.get(&d)).cloned())
                        {
                            // d is 1 when its test holds: d == 1 is the test, d == 0 its negation
                            let holds = (k == 1) == (*op == BinOp::Eq);
                            known = Some((if holds { t } else { negate(t) }, a, z));
                        }
                    }
                    Rvalue::UnaryOp(super::ir::UnOp::Not, Operand::Copy(p) | Operand::Move(p)) => {
                        if let Some((t, a, z)) = p.as_local().and_then(|x| tests.get(&x)).cloned() {
                            known = Some((negate(t), a, z));
                        }
                    }
                    _ => {}
                }
                // what this line writes ends what was known of it, and of every test that reads it
                tests.retain(|x, (_, a, _)| Some(*x) != written && a.place().and_then(|q| q.base_local()) != written);
                if let (Some((op, a, z)), Some(yl)) = (known, y.as_local()) {
                    *rv = Rvalue::BinaryOp(op, a.clone(), z.clone());
                    if a.place().and_then(|q| q.base_local()) != Some(yl) {
                        tests.insert(yl, (op, a, z));
                    }
                }
            }
        }
    }
    let mut bools = vec![];
    for (d, is_sign) in sign_reads {
        let mut switched_only = true;
        for b in &f.body.blocks {
            for s in &b.statements {
                let def = matches!(s, Statement::Assign(p, _) if p.as_local() == Some(d));
                if !def {
                    s.visit_locals(&mut |l| switched_only &= l != d);
                }
            }
            match &b.terminator {
                Terminator::SwitchInt {
                    discr: Operand::Copy(p) | Operand::Move(p),
                    arms,
                    ..
                } if p.as_local() == Some(d) => switched_only &= arms.iter().all(|(v, _)| (0..=1).contains(v)),
                t => t.visit_locals(&mut |l| switched_only &= l != d),
            }
        }
        if !switched_only {
            // read as a number: a sign tag's is the tag plus one (a test made a value costs more)
            if is_sign {
                for st in f.body.blocks.iter_mut().flat_map(|b| b.statements.iter_mut()) {
                    if let Statement::Assign(x, Rvalue::BinaryOp(op @ BinOp::Ge, _, k)) = st {
                        if x.as_local() == Some(d) {
                            *op = BinOp::Add;
                            *k = Operand::Constant(Const::Int {
                                value: 1,
                                ty: IntTy::I16,
                            });
                        }
                    }
                }
            }
            continue;
        }
        for b in f.body.blocks.iter_mut() {
            if let Terminator::SwitchInt {
                discr: Operand::Copy(p) | Operand::Move(p),
                arms,
                otherwise,
            } = &mut b.terminator
            {
                if p.as_local() != Some(d) {
                    continue;
                }
                let to = |v: i128| arms.iter().find(|(a, _)| *a == v).map(|(_, b)| *b);
                let (no, yes) = (to(0).or(*otherwise), to(1).or(*otherwise));
                if let (Some(no), Some(yes)) = (no, yes) {
                    *arms = vec![(0, no)];
                    *otherwise = Some(yes);
                }
            }
        }
        bools.push(d);
    }
    // `x.unwrap_or(-1)` with a niche: the plain variant's arm sets -1, the other's copies the payload, which is -1
    // for the plain variant: so both are the copy, and the switch goes
    for &d in &bools {
        let payload = f.body.blocks.iter().flat_map(|b| &b.statements).find_map(|s| match s {
            Statement::Assign(x, Rvalue::BinaryOp(_, Operand::Copy(p), _)) if x.as_local() == Some(d) => {
                Some(p.clone())
            }
            _ => None,
        });
        let Some(payload) = payload else { continue };
        let block = |name: BasicBlock| f.body.blocks.iter().find(|b| b.name == name);
        // the one line of an arm, `x = v`, and where it goes on
        let arm = |name: BasicBlock| {
            let b = block(name)?;
            let ([Statement::Assign(x, Rvalue::Use(v))], Terminator::Goto(next)) =
                (b.statements.as_slice(), &b.terminator)
            else {
                return None;
            };
            Some((x.clone(), v.clone(), *next))
        };
        let mut folds = vec![];
        for (i, b) in f.body.blocks.iter().enumerate() {
            let Terminator::SwitchInt {
                discr: Operand::Copy(p) | Operand::Move(p),
                arms,
                otherwise: Some(yes),
            } = &b.terminator
            else {
                continue;
            };
            let ([(0, no)], true) = (arms.as_slice(), p.as_local() == Some(d)) else {
                continue;
            };
            let (Some((x0, v0, n0)), Some((x1, v1, n1))) = (arm(*no), arm(*yes)) else {
                continue;
            };
            let minus_one = matches!(v0, Operand::Constant(Const::Int { value: -1, .. }));
            let copies = matches!(&v1, Operand::Copy(q) | Operand::Move(q) if *q == payload);
            if x0 == x1 && n0 == n1 && minus_one && copies {
                folds.push((i, *yes));
            }
        }
        for (i, yes) in folds {
            f.body.blocks[i].terminator = Terminator::Goto(yes);
        }
    }
    // `opt == Some(c)` with a niche: if the payload is negative go to X, else if it is c go to Z, else to X. The
    // plain variant's payload is exactly -1, so that is: if the payload is c (and c is 0 or more) go to Z, else to
    // X. Tested in that order, the test of c is one the range pass decides when c is a counter from 0. The
    // same for `payload >= q` (`Some(e) if e >= s`): compared first, the payload's own sign is then decided by q's
    // range when q is 0 or more
    let mut nb = 750000;
    for &d in &bools {
        let payload = f.body.blocks.iter().flat_map(|b| &b.statements).find_map(|s| match s {
            Statement::Assign(x, Rvalue::BinaryOp(_, Operand::Copy(p), _)) if x.as_local() == Some(d) => {
                Some(p.clone())
            }
            _ => None,
        });
        let Some(payload) = payload else { continue };
        // where a block goes once its empty jumps are followed
        let land = |mut name: BasicBlock| {
            for _ in 0..8 {
                match f.body.blocks.iter().find(|b| b.name == name) {
                    Some(b) if b.statements.is_empty() => match b.terminator {
                        Terminator::Goto(next) => name = next,
                        _ => break,
                    },
                    _ => break,
                }
            }
            name
        };
        let mut swaps = vec![];
        for (i, b) in f.body.blocks.iter().enumerate() {
            let Terminator::SwitchInt {
                discr: Operand::Copy(p) | Operand::Move(p),
                arms,
                otherwise: Some(yes),
            } = &b.terminator
            else {
                continue;
            };
            let ([(0, no)], true) = (arms.as_slice(), p.as_local() == Some(d)) else {
                continue;
            };
            let Some(si) = f.body.blocks.iter().position(|b| b.name == *yes) else {
                continue;
            };
            let some = &f.body.blocks[si];
            // the Some arm: the payload (copied to t, or read as it is) compared with c, and a switch on that
            // plain copies, then the comparison: t, a copy of the payload, if one is
            let Some((cmp, copies)) = some.statements.split_last() else {
                continue;
            };
            let mut t = None;
            let plain = copies.iter().all(|st| match st {
                Statement::Assign(x, Rvalue::Use(Operand::Copy(q) | Operand::Move(q))) if x.as_local().is_some() => {
                    if *q == payload {
                        t = x.as_local();
                    }
                    true
                }
                _ => false,
            });
            if !plain {
                continue;
            }
            let Statement::Assign(
                e,
                Rvalue::BinaryOp(cop @ (BinOp::Eq | BinOp::Ge | BinOp::Gt | BinOp::Le | BinOp::Lt), a, c),
            ) = cmp
            else {
                continue;
            };
            let is_payload = |o: &Operand| match o {
                Operand::Copy(q) | Operand::Move(q) => {
                    Some(q.clone()) == Some(payload.clone()) || (t.is_some() && q.as_local() == t)
                }
                _ => false,
            };
            let (other, payload_left) = if is_payload(a) {
                (c, true)
            } else if is_payload(c) {
                (a, false)
            } else {
                continue;
            };
            if other.local().is_some_and(|l| Some(l) == t) {
                continue;
            }
            // what is tested on the way the comparison holds: for ==, c's sign (the payload is c); for payload >= q or
            // payload > q, the payload's own, which the range pass decides from q's range when q is 0 or more
            let then_test = match (cop, payload_left) {
                (BinOp::Eq, _) => other.clone(),
                (BinOp::Ge | BinOp::Gt, true) | (BinOp::Le | BinOp::Lt, false) => Operand::Copy(payload.clone()),
                _ => continue,
            };
            let Terminator::SwitchInt {
                discr: Operand::Copy(q) | Operand::Move(q),
                arms: earms,
                otherwise: Some(z),
            } = &some.terminator
            else {
                continue;
            };
            let ([(0, ne)], true) = (earms.as_slice(), q.as_local() == e.as_local()) else {
                continue;
            };
            if land(*ne) != land(*no) {
                continue;
            }
            swaps.push((i, si, then_test, *ne, *z));
        }
        for (i, si, c, x, z) in swaps {
            f.body.blocks[i].terminator = Terminator::Goto(f.body.blocks[si].name);
            // the way the comparison holds: then the payload is 0 or more
            let to_z = match c.int() {
                Some((v, _)) if v >= 0 => Some(z),
                Some(_) => Some(x),
                None => None,
            };
            let then = match to_z {
                Some(to) => to,
                None => {
                    let g = fresh(Ty::Bool);
                    let name = BasicBlock(nb);
                    nb += 1;
                    let ty = match c.local().and_then(|l| types.get(&l)) {
                        Some(Ty::Int(t)) => *t,
                        _ => IntTy::I16,
                    };
                    let test = Rvalue::BinaryOp(BinOp::Ge, c.clone(), Operand::Constant(Const::Int { value: 0, ty }));
                    f.body.blocks.push(super::ir::Block {
                        name,
                        statements: vec![Statement::Assign(Place::local(g), test)],
                        terminator: Terminator::SwitchInt {
                            discr: Operand::Move(Place::local(g)),
                            arms: vec![(0, x)],
                            otherwise: Some(z),
                        },
                    });
                    name
                }
            };
            if let Terminator::SwitchInt { otherwise, .. } = &mut f.body.blocks[si].terminator {
                *otherwise = Some(then);
            }
        }
    }
    // and the types: each enum with data is its struct; a discriminant, an isize, is a card
    for (_, t) in f.params.iter_mut() {
        enums.retype(t);
    }
    enums.retype(&mut f.ret);
    for d in f.body.decls.iter_mut() {
        enums.retype(&mut d.ty);
        if bools.contains(&d.local) {
            d.ty = Ty::Bool;
        }
    }
    f.body
        .decls
        .extend(temps.take().into_iter().map(|(local, ty)| super::ir::Decl {
            local,
            mutable: true,
            ty,
        }));
    struct IsizeIsI16;
    impl VisitMut for IsizeIsI16 {
        fn visit_const(&mut self, c: &mut Const) {
            if let Const::Int {
                ty: ty @ IntTy::Isize, ..
            } = c
            {
                *ty = IntTy::I16;
            }
            walk_const(self, c)
        }
        fn visit_ty(&mut self, t: &mut Ty) {
            if *t == Ty::Int(IntTy::Isize) {
                *t = Ty::Int(IntTy::I16);
            }
            walk_ty(self, t)
        }
    }
    IsizeIsI16.visit_body(&mut f.body);
    // a field's type in a place, an enum within a struct (`(_4.1: Option<i16>)`): its struct too
    struct Retype<'a, 'b>(&'a Enums<'b>);
    impl VisitMut for Retype<'_, '_> {
        fn visit_ty(&mut self, t: &mut Ty) {
            self.0.retype(t)
        }
    }
    Retype(enums).visit_body(&mut f.body);
    Ok(())
}

/// What a place is once its enums with data are cells (see `Resolved`), from its local's type through its
/// projections.
fn resolve(p: &Place, types: &HashMap<Local, Ty>, enums: &Enums) -> Result<Resolved, Fail> {
    let Some(l) = p.base_local() else {
        return Ok(Resolved::Plain);
    };
    let Some(mut t) = types.get(&l).cloned() else {
        return Ok(Resolved::Plain);
    };
    let whole_at = |n: usize| Place {
        base: p.base.clone(),
        projection: p.projection[..n].to_vec(),
    };
    // within a whole enum: (the place of the outermost, where its cells start, the layout)
    let mut within: Option<(Place, u32, usize)> = enums.of(&t).map(|l| (whole_at(0), 0, l));
    let mut i = 0;
    while i < p.projection.len() {
        let x = &p.projection[i];
        if let Some((w, at, l)) = &within {
            // a cell already, `(_x.c: T)`: the MIR never projects a field of an enum but through a variant
            if matches!(x, Projection::Field(..)) {
                return Ok(Resolved::Plain);
            }
            let (Projection::Downcast(v), Some(Projection::Field(k, ft))) = (x, p.projection.get(i + 1)) else {
                return Err(format!(
                    "{p}: a projection of an enum with data that isn't a variant's field"
                ));
            };
            let Some((start, kind)) = enums
                .layout(*l)
                .variants
                .get(v)
                .and_then(|(_, at_k)| at_k.get(*k as usize))
            else {
                return Err(format!("{p}: no such variant or field"));
            };
            match kind {
                Kind::Cell(kt) => {
                    let mut c = w.clone();
                    // (a tuple's cell is the struct it is written as)
                    let t = if matches!(ft, Ty::Tuple(_)) {
                        kt.clone()
                    } else {
                        ft.clone()
                    };
                    c.projection.push(Projection::Field(at + start, t));
                    c.projection.extend(p.projection[i + 2..].iter().cloned());
                    return Ok(Resolved::Cell(c));
                }
                Kind::Enum(j) => within = Some((w.clone(), at + start, *j)),
                Kind::Never => return Err(format!("{p}: a field of a type with no values")),
                // (rustc writes `const ()` where a `()` is used, so the MIR has no reason to read one)
                Kind::Unit => return Err(format!("{p}: a field of type ()")),
            }
            i += 2;
            continue;
        }
        t = match (x, t) {
            (Projection::Deref, Ty::Ref { to, .. } | Ty::Ptr { to, .. }) => *to,
            (Projection::Field(_, ft), _) => ft.clone(),
            (
                Projection::Index(_)
                | Projection::IndexPlus(..)
                | Projection::ConstantIndex { .. }
                | Projection::IndexPlace(_),
                Ty::Array(e, _) | Ty::Slice(e),
            ) => *e,
            _ => return Ok(Resolved::Plain),
        };
        i += 1;
        if let Some(l) = enums.of(&t) {
            within = Some((whole_at(i), 0, l));
        }
    }
    Ok(match within {
        Some((w, at, l)) => Resolved::Whole(w, at, l),
        None => Resolved::Plain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_enum_is_found_by_the_mir_s_name_for_it() {
        let def = |path: &[&str], variants: &[(&str, i128, usize)]| EnumDef {
            path: path.iter().map(|p| p.to_string()).collect(),
            args: vec![],
            in_crate: true,
            variants: variants
                .iter()
                .map(|&(name, value, fields)| Variant {
                    name: name.to_string(),
                    value,
                    fields: vec![Ty::Int(IntTy::I16); fields],
                })
                .collect(),
        };
        let d = [
            def(&["paint", "Colour"], &[("Red", 0, 0), ("Gloss", -1, 0)]),
            def(&["light", "Colour"], &[("Infra", 0, 0)]),
            def(&["Shape"], &[("Circle", 0, 1)]),
        ];
        assert_eq!(d[0].variants[1].value, -1);
        assert!(d[0].c_like() && !d[2].c_like());
        assert_eq!(find(&d, &["paint", "Colour"], &[]).unwrap().path, ["paint", "Colour"]);
        // a bare name two enums share finds neither: the MIR names such an enum by its whole path
        assert!(find(&d, &["Colour"], &[]).is_none());
        assert_eq!(find(&d, &["Shape"], &[]).unwrap().path, ["Shape"]);
    }
}
