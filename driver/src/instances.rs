//! A generic function's instances, each compiled as a function of its own. rustc writes a generic function's MIR
//! once, in its generic form (`fn bigger(_1: T, _2: T) -> T`), and where it doesn't inline a call to it the call
//! names an instance (`bigger::<i16>`), which has no MIR of its own: codegen instantiates the generic body for it.
//! So does the driver: the generic body with the instance's arguments in place of its parameters is an ordinary
//! function, named here by the instance (`bigger__i16`), and the calls to the instance are calls to it.
//!
//! Only the program's own generic functions (and its traits' default methods, generic over `Self`) are made so; an
//! instance whose body calls others is followed to those. Another crate's stay refused, as the linker says.
use std::collections::{HashMap, HashSet};

use cargo_cardiac::compile::{ir, BUILTINS};
use rustc_middle::mir as mmir;
use rustc_middle::ty::{self as mty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

/// More instances than this is a generic function instantiated without end (`f::<T>` calling `f::<[T; 2]>`): the
/// rest stay calls, refused.
const MOST: usize = 64;

/// The instances the program's calls reach.
pub struct Instances<'tcx> {
    /// Each instance's name and its body, instantiated, in the order they were found.
    pub bodies: Vec<(ir::ItemPath, &'tcx mmir::Body<'tcx>)>,
    /// The functions the instances' calls reach (an impl's method `<Sq as Area>::area`), for `convert::items_reached`
    /// to name as calls name them.
    pub called: HashSet<DefId>,
    /// A call's callee, as the IR has it (`bigger::<i16>`), to the name of the instance it calls.
    calls: HashMap<ir::Path, String>,
}

/// The instances of the program's generic functions that the calls in `defs`, its items, reach, and those their
/// calls reach in turn.
pub fn find<'tcx>(tcx: TyCtxt<'tcx>, defs: &[DefId]) -> Instances<'tcx> {
    let mut found = Instances {
        bodies: vec![],
        called: HashSet::new(),
        calls: HashMap::new(),
    };
    let mut names: HashMap<mty::Instance<'tcx>, String> = HashMap::new();
    // the names the compiler gives the program's own functions (`arith__twice`, `P_as_Twice__twice`)
    let mut used: HashSet<String> = defs
        .iter()
        .filter(|&&d| {
            matches!(
                tcx.def_kind(d),
                rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
            )
        })
        .map(|&d| mangled(&tcx.def_path_str(d)))
        .collect();
    let mut refused: HashSet<mty::Instance<'tcx>> = HashSet::new();
    let mut todo: Vec<&mmir::Body<'tcx>> = defs
        .iter()
        .filter(|&&d| tcx.trivial_const(d).is_none() && !tcx.is_const_fn(d))
        .map(|&d| tcx.instance_mir(mty::InstanceKind::Item(d)))
        .collect();
    while let Some(body) = todo.pop() {
        let env = mty::TypingEnv::post_analysis(tcx, body.source.def_id());
        for b in body.basic_blocks.iter() {
            let mmir::TerminatorKind::Call { func, .. } = &b.terminator().kind else {
                continue;
            };
            let Some((callee, args)) = func.const_fn_def() else {
                continue;
            };
            if args.has_non_region_param() {
                continue;
            }
            let Ok(Some(instance)) = mty::Instance::try_resolve(tcx, env, callee, args) else {
                continue;
            };
            if !wanted(tcx, instance) {
                continue;
            }
            let Ok(path) = ir::parse_path(&format!("{func:?}")) else {
                continue;
            };
            if let Some(name) = names.get(&instance) {
                found.calls.insert(path, name.clone());
                continue;
            }
            if names.len() == MOST || refused.contains(&instance) {
                continue;
            }
            let mut body = instance.instantiate_mir_and_normalize_erasing_regions(
                tcx,
                mty::TypingEnv::fully_monomorphized(),
                mty::EarlyBinder::bind(tcx.instance_mir(instance.def).clone()),
            );
            if !codegen_view(tcx, &mut body) {
                refused.insert(instance);
                continue;
            }
            let name = unused(
                &mangled(&tcx.def_path_str_with_args(instance.def_id(), instance.args)),
                &used,
            );
            used.insert(name.clone());
            names.insert(instance, name.clone());
            found.calls.insert(path, name.clone());
            let body: &'tcx mmir::Body<'tcx> = tcx.arena.alloc(body);
            found.bodies.push((ir::ItemPath { segments: vec![name] }, body));
            todo.push(body);
        }
    }
    for &(_, body) in &found.bodies {
        crate::convert::calls_in(tcx, body, &mut found.called);
    }
    found
}

/// Whether an instance is one made here: of the program's own function (not a closure's, nor a built-in, which is
/// an instruction whatever its arguments: `load_resource::<3>`) or `Ord`'s own `max` or `min` on a C-like enum,
/// generic, with MIR, and with no promoted constant (those are the generic body's, which it would share).
fn wanted<'tcx>(tcx: TyCtxt<'tcx>, instance: mty::Instance<'tcx>) -> bool {
    use rustc_hir::def::DefKind;
    let mty::InstanceKind::Item(def) = instance.def else {
        return false;
    };
    let own = def.is_local() && tcx.opt_item_name(def).is_some_and(|n| !BUILTINS.contains(&n.as_str()));
    (own || ord_of_c_like(tcx, instance))
        && matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn)
        && tcx.generics_of(def).requires_monomorphization(tcx)
        && tcx.is_mir_available(def)
        && tcx.promoted_mir(def).is_empty()
}

/// Whether an instance is core's `Ord::max` or `Ord::min` (the trait's own, `if other < self { self } else { other
/// }`) on a C-like enum or a struct of plain fields: its `<` is the type's, which `operator` reads when it is derived.
fn ord_of_c_like<'tcx>(tcx: TyCtxt<'tcx>, instance: mty::Instance<'tcx>) -> bool {
    let def = instance.def_id();
    (tcx.is_diagnostic_item(rustc_span::sym::cmp_ord_max, def)
        || tcx.is_diagnostic_item(rustc_span::sym::cmp_ord_min, def))
        && (c_like(instance.args.type_at(0)) || derived_fields(tcx, instance.args.type_at(0)).is_some())
}

/// The fields' types of a type that is one of the program's structs whose `PartialOrd` is derived, each an integer,
/// a bool or a char: the derived `partial_cmp` compares the fields in turn, as those compare, and stops at the first
/// that differ. None for any other type.
fn derived_fields<'tcx>(tcx: TyCtxt<'tcx>, ty: mty::Ty<'tcx>) -> Option<Vec<mty::Ty<'tcx>>> {
    let mty::Adt(adt, args) = ty.kind() else { return None };
    if !adt.is_struct() || !adt.did().is_local() || !args.is_empty() {
        return None;
    }
    let tr = tcx.lang_items().partial_ord_trait()?;
    let &partial_cmp = tcx
        .associated_item_def_ids(tr)
        .iter()
        .find(|&&d| tcx.item_name(d).as_str() == "partial_cmp")?;
    let env = mty::TypingEnv::fully_monomorphized();
    let used = mty::Instance::try_resolve(tcx, env, partial_cmp, tcx.mk_args(&[ty.into(), ty.into()])).ok()??;
    if !tcx.impl_of_assoc(used.def_id()).is_some_and(|i| tcx.is_automatically_derived(i)) {
        return None;
    }
    let fields: Vec<mty::Ty<'tcx>> = adt
        .non_enum_variant()
        .fields
        .iter()
        .map(|f| f.ty(tcx, args).skip_normalization())
        .collect();
    let plain = |t: &mty::Ty<'_>| t.is_integral() || t.is_bool() || t.is_char();
    (!fields.is_empty() && fields.iter().all(plain)).then_some(fields)
}

/// Whether a type is an enum of variants without fields (and at least one), which is its discriminant.
fn c_like(ty: mty::Ty<'_>) -> bool {
    match ty.kind() {
        mty::Adt(adt, _) => {
            adt.is_enum() && !adt.variants().is_empty() && adt.variants().iter().all(|v| v.fields.is_empty())
        }
        _ => false,
    }
}

/// An instance's body made what codegen makes of it, where the compiler couldn't read the generic form: what the
/// generic body does through a trait, on a type now known, done as rustc does it for that type. False (the instance
/// stays a call, refused) if it drops a value that needs dropping.
///
/// - A `drop` of a value that needs none (an `i16`, a struct of them) is no drop.
/// - A comparison, `clone`, `+`, `-` or `*` of integers (or a comparison of bools), which a generic body calls
///   through its trait (`<T as PartialOrd>::gt(move _4, move _5)`), is the operation, as rustc writes it in a
///   function that isn't generic: `Gt(copy (*_4), copy (*_5))`. The arithmetic is checked as the program's overflow
///   checks say, as core's (`#[rustc_inherit_overflow_checks]`) is.
/// - An unwind to a cleanup block continues instead: with no drop, cleanup does nothing, and a panic ends the run.
/// - A constant the generic body names through its parameters (`<T as Limit>::MAX`, an integer or a bool) is its
///   value. Any other such constant, which the compiler would look up by a name it doesn't know, leaves the instance
///   a call.
fn codegen_view<'tcx>(tcx: TyCtxt<'tcx>, body: &mut mmir::Body<'tcx>) -> bool {
    use mmir::visit::MutVisitor;
    use mmir::TerminatorKind as T;
    let env = mty::TypingEnv::fully_monomorphized();
    let mut values = Values { tcx, failed: false };
    values.visit_body(body);
    if values.failed {
        return false;
    }
    for k in 0..body.basic_blocks.len() {
        let bb = mmir::BasicBlock::from_usize(k);
        if body.basic_blocks[bb].is_cleanup {
            continue;
        }
        let kind = body.basic_blocks[bb].terminator().kind.clone();
        match kind {
            T::Drop { place, target, .. } => {
                if place.ty(&body.local_decls, tcx).ty.needs_drop(tcx, env) {
                    return false;
                }
                body.basic_blocks_mut()[bb].terminator_mut().kind = T::Goto { target };
            }
            T::Call {
                func,
                args,
                destination,
                target: Some(target),
                ..
            } => {
                if let Some(op) = operator(tcx, &func) {
                    let args: Vec<mmir::Operand<'tcx>> = args.iter().map(|a| a.node.clone()).collect();
                    if !operation(tcx, body, bb, op, &args, destination, target) {
                        return false;
                    }
                }
            }
            _ => {}
        }
        if let Some(unwind) = body.basic_blocks_mut()[bb].terminator_mut().unwind_mut() {
            if matches!(unwind, mmir::UnwindAction::Cleanup(_)) {
                *unwind = mmir::UnwindAction::Continue;
            }
        }
    }
    true
}

/// Each constant named through the instance's arguments evaluated (`codegen_view`).
struct Values<'tcx> {
    tcx: TyCtxt<'tcx>,
    /// Whether one couldn't be.
    failed: bool,
}

impl<'tcx> mmir::visit::MutVisitor<'tcx> for Values<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_const_operand(&mut self, c: &mut mmir::ConstOperand<'tcx>, _: mmir::Location) {
        let mmir::Const::Unevaluated(u, ty) = c.const_ else {
            return;
        };
        if u.args.is_empty() {
            return;
        }
        if !(ty.is_integral() || ty.is_bool()) {
            self.failed = true;
            return;
        }
        match c.const_.eval(self.tcx, mty::TypingEnv::fully_monomorphized(), c.span) {
            Ok(v) => c.const_ = mmir::Const::Val(v, ty),
            Err(_) => self.failed = true,
        }
    }
}

/// An operation of a trait core implements for integers and bools, as a call names it.
#[derive(Clone, Copy)]
enum Operator {
    /// `PartialOrd::lt` and the rest, `PartialEq::eq` and `ne`: on two references.
    Compare(mmir::BinOp),
    /// `Clone::clone`, of a reference.
    Clone,
    /// `Add::add`, `Sub::sub`, `Mul::mul`: on two values.
    Arith(mmir::BinOp),
    /// A comparison of two references to a C-like enum whose `PartialOrd` (or `PartialEq`) is derived: its
    /// discriminants compared, as the derived `partial_cmp` (or `eq`) compares them.
    Discriminants(mmir::BinOp),
    /// `PartialOrd::lt` and the rest on two references to a struct whose `PartialOrd` is derived (`derived_fields`):
    /// its fields compared in turn.
    Fields(mmir::BinOp),
}

/// The operation a call is, if it calls one of core's operator traits' methods on integers (comparisons on bools
/// too), with both sides of the same type: core implements those as the operation, and nothing else can implement
/// them. Or a comparison of a C-like enum whose comparison is derived (`derived_compare`).
fn operator<'tcx>(tcx: TyCtxt<'tcx>, func: &mmir::Operand<'tcx>) -> Option<Operator> {
    use mmir::BinOp as B;
    let (callee, args) = func.const_fn_def()?;
    let tr = tcx.trait_of_assoc(callee)?;
    let lang = tcx.lang_items();
    let this = args.type_at(0);
    let same = || args.types().nth(1) == Some(this);
    let name = tcx.item_name(callee);
    let compare = this.is_integral() || this.is_bool();
    let ordered = Some(tr) == lang.partial_ord_trait() && same();
    let equal = Some(tr) == lang.eq_trait() && same();
    let op = match name.as_str() {
        "lt" if ordered => B::Lt,
        "le" if ordered => B::Le,
        "gt" if ordered => B::Gt,
        "ge" if ordered => B::Ge,
        "eq" if equal => B::Eq,
        "ne" if equal => B::Ne,
        _ => B::Cmp,
    };
    Some(match name.as_str() {
        _ if op != B::Cmp && compare => Operator::Compare(op),
        _ if op != B::Cmp && derived_compare(tcx, callee, args) => Operator::Discriminants(op),
        _ if ordered && op != B::Cmp && own_method(tcx, callee, args) && derived_fields(tcx, this).is_some() => {
            Operator::Fields(op)
        }
        "clone" if Some(tr) == lang.clone_trait() && compare => Operator::Clone,
        "add" if Some(tr) == lang.add_trait() && this.is_integral() && same() => Operator::Arith(B::Add),
        "sub" if Some(tr) == lang.sub_trait() && this.is_integral() && same() => Operator::Arith(B::Sub),
        "mul" if Some(tr) == lang.mul_trait() && this.is_integral() && same() => Operator::Arith(B::Mul),
        _ => return None,
    })
}

/// Whether a call of one of a trait's methods runs the trait's own (its default), not an impl's.
fn own_method<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: mty::GenericArgsRef<'tcx>) -> bool {
    let env = mty::TypingEnv::fully_monomorphized();
    mty::Instance::try_resolve(tcx, env, callee, args)
        .ok()
        .flatten()
        .is_some_and(|i| i.def_id() == callee)
}

/// Whether a call of a comparison of `PartialOrd` or `PartialEq` (`callee`, one of the trait's own) on a C-like enum
/// is the derived one: the method is the trait's own (`lt` is `partial_cmp`'s result tested, `ne` is `!eq`), and the
/// method it uses (`partial_cmp`, `eq`) is the impl's, which `#[derive]` wrote. That one compares the discriminants.
fn derived_compare<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: mty::GenericArgsRef<'tcx>) -> bool {
    let env = mty::TypingEnv::fully_monomorphized();
    if !c_like(args.type_at(0)) {
        return false;
    }
    let Some(tr) = tcx.trait_of_assoc(callee) else {
        return false;
    };
    let resolved = |def: DefId| mty::Instance::try_resolve(tcx, env, def, args).ok().flatten();
    let base = if Some(tr) == tcx.lang_items().partial_ord_trait() {
        "partial_cmp"
    } else {
        "eq"
    };
    let Some(&used) = tcx
        .associated_item_def_ids(tr)
        .iter()
        .find(|&&d| tcx.item_name(d).as_str() == base)
    else {
        return false;
    };
    let Some(call) = resolved(callee) else { return false };
    let Some(used) = resolved(used) else { return false };
    // the method called is the trait's own unless it is the one used (`eq`)
    let own = call.def_id() == callee || call.def_id() == used.def_id();
    let derived = tcx
        .impl_of_assoc(used.def_id())
        .is_some_and(|i| tcx.is_automatically_derived(i));
    own && derived
}

/// Block `bb`'s call of `op` made the operation, its result in `dest`, then on to `target`. False if an argument
/// to deref is a constant, not a place.
fn operation<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mut mmir::Body<'tcx>,
    bb: mmir::BasicBlock,
    op: Operator,
    args: &[mmir::Operand<'tcx>],
    dest: mmir::Place<'tcx>,
    target: mmir::BasicBlock,
) -> bool {
    use mmir::{Operand, Rvalue, StatementKind};
    let source_info = body.basic_blocks[bb].terminator().source_info;
    let copied = |o: &Operand<'tcx>| match o {
        Operand::Move(p) => Operand::Copy(*p),
        o => o.clone(),
    };
    let deref = |o: &Operand<'tcx>| o.place().map(|p| Operand::Copy(tcx.mk_place_deref(p)));
    let assign = |place, rvalue| mmir::Statement::new(source_info, StatementKind::Assign(Box::new((place, rvalue))));
    let goto = mmir::TerminatorKind::Goto { target };
    match (op, args) {
        (Operator::Compare(bin), [a, b]) => {
            let (Some(a), Some(b)) = (deref(a), deref(b)) else {
                return false;
            };
            let block = &mut body.basic_blocks_mut()[bb];
            block
                .statements
                .push(assign(dest, Rvalue::BinaryOp(bin, Box::new((a, b)))));
            block.terminator_mut().kind = goto;
        }
        (Operator::Discriminants(bin), [a, b]) => {
            // `_x = discriminant((*a)); _y = discriminant((*b)); dest = Lt(move _x, move _y)`
            let (Some(a), Some(b)) = (a.place(), b.place()) else {
                return false;
            };
            let Some(enum_ty) = a.ty(&body.local_decls, tcx).ty.builtin_deref(true) else {
                return false;
            };
            let d = enum_ty.discriminant_ty(tcx);
            let x = mmir::Place::from(body.local_decls.push(mmir::LocalDecl::new(d, source_info.span)));
            let y = mmir::Place::from(body.local_decls.push(mmir::LocalDecl::new(d, source_info.span)));
            let block = &mut body.basic_blocks_mut()[bb];
            block
                .statements
                .push(assign(x, Rvalue::Discriminant(tcx.mk_place_deref(a))));
            block
                .statements
                .push(assign(y, Rvalue::Discriminant(tcx.mk_place_deref(b))));
            block.statements.push(assign(
                dest,
                Rvalue::BinaryOp(bin, Box::new((Operand::Move(x), Operand::Move(y)))),
            ));
            block.terminator_mut().kind = goto;
        }
        (Operator::Fields(bin), [a, b]) => {
            // field by field, as the derived `partial_cmp` stops at the first that differ: `a.k < b.k` is true (for
            // `lt` and `le`), `b.k < a.k` false, and only the last field is compared with `bin` itself. Each answer
            // is a constant, which the compiler takes straight to the test it goes to.
            let (Some(a), Some(b)) = (a.place(), b.place()) else {
                return false;
            };
            let Some(fields) = a
                .ty(&body.local_decls, tcx)
                .ty
                .builtin_deref(true)
                .and_then(|t| derived_fields(tcx, t))
            else {
                return false;
            };
            let field = |p: mmir::Place<'tcx>, k: usize| {
                Operand::Copy(tcx.mk_place_field(tcx.mk_place_deref(p), rustc_abi::FieldIdx::from_usize(k), fields[k]))
            };
            let strict = match bin {
                mmir::BinOp::Lt | mmir::BinOp::Le => mmir::BinOp::Lt,
                _ => mmir::BinOp::Gt,
            };
            let block = |stmts, kind| {
                mmir::BasicBlockData::new_stmts(stmts, Some(mmir::Terminator { source_info, kind }), false)
            };
            let answer = |v: bool| {
                let c = Operand::Constant(Box::new(mmir::ConstOperand {
                    span: source_info.span,
                    user_ty: None,
                    const_: mmir::Const::from_bool(tcx, v),
                }));
                block(vec![assign(dest, Rvalue::Use(c, mmir::WithRetag::Yes))], goto.clone())
            };
            let yes = body.basic_blocks_mut().push(answer(true));
            let no = body.basic_blocks_mut().push(answer(false));
            // a block of `_t = op(x, y); switchInt(move _t) -> [0: otherwise, else: then]`
            let test = |body: &mut mmir::Body<'tcx>, op, x, y, then, otherwise| {
                let t = mmir::LocalDecl::new(tcx.types.bool, source_info.span);
                let t = mmir::Place::from(body.local_decls.push(t));
                let switch = mmir::TerminatorKind::SwitchInt {
                    discr: Operand::Move(t),
                    targets: mmir::SwitchTargets::new(std::iter::once((0, otherwise)), then),
                };
                let stmt = assign(t, Rvalue::BinaryOp(op, Box::new((x, y))));
                body.basic_blocks_mut().push(block(vec![stmt], switch))
            };
            let last = fields.len() - 1;
            let mut next = test(body, bin, field(a, last), field(b, last), yes, no);
            for k in (0..last).rev() {
                let loses = test(body, strict, field(b, k), field(a, k), no, next);
                next = test(body, strict, field(a, k), field(b, k), yes, loses);
            }
            body.basic_blocks_mut()[bb].terminator_mut().kind = mmir::TerminatorKind::Goto { target: next };
        }
        (Operator::Clone, [a]) => {
            let Some(a) = deref(a) else { return false };
            let block = &mut body.basic_blocks_mut()[bb];
            block
                .statements
                .push(assign(dest, Rvalue::Use(a, mmir::WithRetag::Yes)));
            block.terminator_mut().kind = goto;
        }
        (Operator::Arith(bin), [a, b]) if !tcx.sess.overflow_checks() => {
            let block = &mut body.basic_blocks_mut()[bb];
            block
                .statements
                .push(assign(dest, Rvalue::BinaryOp(bin, Box::new((a.clone(), b.clone())))));
            block.terminator_mut().kind = goto;
        }
        (Operator::Arith(bin), [a, b]) => {
            // `_t = AddWithOverflow(a, b); assert(!move (_t.1: bool), ..) -> bbN`, and in bbN `dest = move (_t.0)`
            let ty = a.ty(&body.local_decls, tcx);
            let pair = mty::Ty::new_tup(tcx, &[ty, tcx.types.bool]);
            let t = mmir::Place::from(body.local_decls.push(mmir::LocalDecl::new(pair, source_info.span)));
            let checked = match bin {
                mmir::BinOp::Add => mmir::BinOp::AddWithOverflow,
                mmir::BinOp::Sub => mmir::BinOp::SubWithOverflow,
                _ => mmir::BinOp::MulWithOverflow,
            };
            let field = |k: usize, ty| Operand::Move(tcx.mk_place_field(t, rustc_abi::FieldIdx::from_usize(k), ty));
            let after = mmir::BasicBlockData::new_stmts(
                vec![assign(dest, Rvalue::Use(field(0, ty), mmir::WithRetag::Yes))],
                Some(mmir::Terminator {
                    source_info,
                    kind: goto,
                }),
                false,
            );
            let after = body.basic_blocks_mut().push(after);
            let block = &mut body.basic_blocks_mut()[bb];
            block
                .statements
                .push(assign(t, Rvalue::BinaryOp(checked, Box::new((copied(a), copied(b))))));
            block.terminator_mut().kind = mmir::TerminatorKind::Assert {
                cond: field(1, tcx.types.bool),
                expected: false,
                msg: Box::new(mmir::AssertKind::Overflow(bin, copied(a), copied(b))),
                target: after,
                unwind: mmir::UnwindAction::Continue,
            };
        }
        _ => return false,
    }
    true
}

/// An instance's name as one plain name: rustc's (`bigger::<i16>`, `<A as Shown>::show`), each run of what isn't a
/// letter, a digit or `_` one `_`, or `__` where it holds a `::` (`bigger__i16`, `A_as_Shown__show`).
fn mangled(printed: &str) -> String {
    let mut out = String::new();
    let mut gap = String::new();
    for c in printed.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if !out.is_empty() && !gap.is_empty() {
                out.push_str(if gap.contains("::") { "__" } else { "_" });
            }
            gap.clear();
            out.push(c);
        } else {
            gap.push(c);
        }
    }
    out
}

/// `name`, or with a number after it if that is taken (`f::<&i16>` and `f::<i16>` both mangle to `f__i16`).
fn unused(name: &str, used: &HashSet<String>) -> String {
    let mut candidate = name.to_string();
    let mut k = 2;
    while used.contains(&candidate) {
        candidate = format!("{name}_{k}");
        k += 1;
    }
    candidate
}

impl Instances<'_> {
    /// Each call to an instance in `file` made a call to the instance's function, by its name.
    pub fn rename_calls(&self, file: &mut ir::File) {
        for b in file.bodies_mut().flat_map(|b| &mut b.blocks) {
            let ir::Terminator::Call { func, .. } = &mut b.terminator else {
                continue;
            };
            if let Some(name) = self.calls.get(func) {
                *func = ir::Path {
                    qself: None,
                    segments: vec![ir::Segment::Name {
                        name: name.clone(),
                        args: None,
                    }],
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instance_is_named_by_plain_words() {
        assert_eq!(mangled("bigger::<i16>"), "bigger__i16");
        assert_eq!(mangled("<A as Shown>::show"), "A_as_Shown__show");
        assert_eq!(mangled("sum::<3>"), "sum__3");
        assert_eq!(mangled("f::<[i16; 3]>"), "f__i16_3");
        let used = HashSet::from(["f__i16".to_string()]);
        assert_eq!(unused("f__i16", &used), "f__i16_2");
    }
}
